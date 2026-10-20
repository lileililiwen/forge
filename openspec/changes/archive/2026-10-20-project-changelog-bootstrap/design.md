## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The scaffold's rendered file list | `forge` | Yes. `src/generate/`. |
| The release lane and its contract | `forge` | Read only. |
| The doctor's maturity assessment | `forge` | Read only. |
| The profile catalog and the profile descriptors | `forge` | Read only. |

## The Gap

The release lane requires a changelog file:

```rust
// src/release/prepare.rs
if !path.is_file() {
    return Err(ForgeError::ReleaseInvalid {
        reason: format!(
            "changelog file `{}` does not exist; release requires a present changelog",
            path.display()
        ),
    });
}
```

That is the right rule. The release captures the changelog, hashes it, and
commits it with the release — a release with no record of what changed is not
a release worth shipping.

The scaffold writes a manifest, a build definition, a container file, a
README, a `.gitignore` and (optionally) the workspace metadata. It writes no
changelog. So a fresh project cannot prepare a release until its owner
hand-writes a file Forge declined to write.

## The File

The scaffold writes `CHANGELOG.md` at the project's root, in the same shape
the repository's own changelog uses:

```markdown
# Changelog

All notable changes to <project name> are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial Forge scaffold for the `<profile>` profile.
```

Three decisions inside that file:

- **No version heading.** The project has not been released. The generated
  language manifests declare `0.1.0`, but that is the project's version, not
  a claim that `0.1.0` shipped. The owner adds the version heading when a
  release is cut.
- **No timestamp.** The scaffold's assembly is deterministic: the same
  creation request renders byte-identical files. A date would break that, so
  the file carries none.
- **One `## [Unreleased]` section, with the scaffold named under it.** The
  owner adds their own entries there.

## Where It Is Written

`template_files()` in `src/generate/engine.rs` builds the profile's files,
then pushes the manifest, then the optional staged files, and finally sorts
the list by path. The changelog is profile-independent apart from the profile
name it mentions, so it is pushed **once**, after the profile match — not
once per profile branch.

The file joins the staged set like every other template file, so the
generation's staging, promotion and cleanup guarantees cover it identically:
a refused generation leaves no directory, and a promoted tree carries the
file.

## The Release Lane Is Untouched

The file lands at `CHANGELOG.md`, which is the release lane's default
changelog path (`crate::release::contract::DEFAULT_CHANGELOG`). Nothing in
`src/release/` changes: the lane still refuses a project with no changelog,
still captures the file's hash and excerpt, and still commits it with the
release.

The canonical scenario from `project-manifest-completeness` — "a project
without a changelog is refused for that reason" — stays true. A project
created before this change, or one whose owner deleted the file, is still
refused, and the refusal still names the missing file.

## User Experience and Interface

`forge new` writes one more file. `forge release prepare` on a fresh project
no longer stops at "changelog file `…/CHANGELOG.md` does not exist".

Nothing else changes: the manifest, the build definition, the container file,
the README and the workspace metadata are byte-identical to what they were.

## Behavioral Model

| Situation | Outcome |
|---|---|
| `forge new` runs for any profile | The tree carries `CHANGELOG.md` with an `## [Unreleased]` section naming the scaffold. |
| The same request runs twice | The two trees are byte-identical; the changelog carries no timestamp. |
| `forge release prepare` runs on a fresh project | The changelog is found; the plan proceeds to the checks. |
| A project has no changelog | The release lane refuses, naming the missing file. Unchanged. |
| The owner cuts a release | The owner adds the version heading; the release commits the file. |

## Contract and Compatibility

The generated tree gains one file. Every other file is byte-identical, and the
scaffold's determinism, its portability (the generated project still builds
with its own toolchain and no Forge runtime) and its workspace metadata are
unchanged.

The release contract is unchanged. The doctor's finding inventory is
unchanged. The `deterministic-project-generation` capability's requirements
are unchanged: the file list is not enumerated there, and every scenario
still holds.

## Failure and Boundary Policy

- **The file is written like every other template file.** It is staged,
  promoted and cleaned up by the existing generation guarantees. A refused
  generation leaves no `CHANGELOG.md` behind either.
- **The file claims nothing.** No version heading, no date, no release
  statement. It names the scaffold and nothing else.
- **No profile is exempt.** Every profile the scaffold supports gets the
  file; the content is the same shape for all of them.
- **Nothing else in the tree is touched.** The changelog does not alter the
  manifest, the language manifest's version, or the workspace metadata.

## Verification Oracle

```bash
# The Forge Core test suite
cargo test --workspace --all-targets -- \
  --skip rust_scaffold_builds_and_tests_with_native_toolchain \
  --skip rust_scaffold_builds_with_native_toolchain_without_forge \
  --skip declaration_is_inert_for_native_builds_present_and_absent \
  --test-threads=1

# The repository Gate
./target/debug/forge gate --dry-run
./target/debug/forge gate --timeout-secs 1800

# Strict OpenSpec validation and whitespace
openspec validate --all --strict --no-interactive
node scripts/check-openspec-change-names.mjs
git diff --check
```

A real run: `forge new` into a temporary directory, then `forge release
prepare` for that project, with the project registered. The plan must name
the changelog it captured.

## Implementation Plan

**The renderer.** A changelog renderer next to the README renderer in
`src/generate/request.rs`: the project name and the profile in, the file's
bytes out.

**The file.** `src/generate/engine.rs` pushes `CHANGELOG.md` once, after the
profile match, so every profile gets it.

**The tests.** The scaffold's own suite carries the new file; a contract test
shows a generated project can prepare a release.

## Decision Ledger

- **Resolved:** the changelog names no version. The project's version is
  `0.1.0` in the language manifest, but a version heading in the changelog
  would claim a release that never happened. The owner adds it.
- **Resolved:** the file carries no timestamp. The scaffold's determinism is
  a contract, and a date would break it.
- **Resolved:** the file is written once, not per profile. It is the same
  file for every profile apart from the profile name it mentions.
- **Resolved:** the doctor's `L3-release` control is not tightened here. It
  counts release evidence by the presence of a changelog file, and that rule
  governs every project rather than the scaffold. A scaffolded project
  targets L1, so the control is not applicable to it; raising the target to
  L3 is the operator's claim, and the rule that counts an unreleased
  changelog as release evidence is a separate change.
- **Implementation authorized (2026-10-20):** the owner directed the trunk
  repair programme and authorized this package in the approved order, after
  `spec-purpose-completion` closed the specifications' purposes.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| A scaffolded project carries a changelog | The tree carries `CHANGELOG.md` | `forge release prepare` no longer stops at a missing file | Written once, after the profile match | `src/generate/` | Every profile | 3.1–3.2 | The scaffold suite |
| The changelog claims no release and no clock | No version heading, no date | The owner adds the heading when a release is cut | Deterministic bytes | `src/generate/request.rs` | Two identical requests | 3.3 | The determinism test |
