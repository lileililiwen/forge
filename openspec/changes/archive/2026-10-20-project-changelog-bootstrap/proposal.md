# Change: project-changelog-bootstrap

## Why

`forge release prepare` refuses a project that has no changelog file. That
refusal is right: the release lane captures the changelog, hashes it, and
commits it with the release, so a release without one is a release with no
record.

But the project Forge scaffolds cannot satisfy it. `forge new` writes a
manifest, a build definition, a container file, a README and a `.gitignore` —
and no changelog. So the first thing an owner does with a fresh project is hit
a refusal that asks for a file Forge itself declined to write, and the owner
hand-writes one before the first release can even be prepared.

The refusal came from the `project-manifest-completeness` change, which moved
the release's file requirement from a missing manifest block to the missing
file it names. That was the right place to draw the line. The gap it left is
here: the scaffold and the release lane disagree about what a project starts
with.

## What Changes

- **The scaffold writes an initial `CHANGELOG.md`.** Every generated project
  carries one, in keep-a-changelog form, with an `## [Unreleased]` section
  that names what the scaffold created.
- **The file claims no release.** There is no version heading, because
  nothing has been released: the project is new. The owner's own entries go
  under `## [Unreleased]`, and the version heading is added when a release
  is cut.
- **The bytes are deterministic.** The file carries no timestamp, so the same
  creation request renders the same tree, exactly like every other scaffold
  file.
- **The release lane is untouched.** The file lands at the path the release
  lane reads by default. Nothing about the release contract changes.

## Package Boundary

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `project-changelog-bootstrap` | forge | Rust | implemented | this package |
| `deterministic-project-generation` | forge | Rust | read only | the scaffold's determinism and portability are unchanged |
| `project-manifest-completeness` | forge | Rust | read only | the refusal stays; the scaffold now satisfies it |
| The release lane and its contract | forge | Rust | read only | not edited |
| The doctor's maturity assessment | forge | Rust | read only | not edited; see the note below |

## Non-Goals

- **The release lane does not change.** It still refuses a project with no
  changelog. A project created before this change, or one whose owner
  removed the file, is still refused, and the refusal still names the file.
- **The scaffold does not invent a version.** The generated language manifests
  already declare `0.1.0`; the changelog names no version, because no version
  has been released.
- **The doctor's maturity rules do not change.** The `L3-release` control
  counts release evidence by the presence of a changelog file, and that rule
  is not touched here — see the handoff note.
- **No existing project is given a changelog.** The change is about what a
  scaffold writes, not about back-filling the workspace.
