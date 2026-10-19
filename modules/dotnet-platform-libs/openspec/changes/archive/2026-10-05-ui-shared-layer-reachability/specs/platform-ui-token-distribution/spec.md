# platform-ui-token-distribution Specification

## ADDED Requirements

### Requirement: The UI token layer SHALL be resolvable by a consumer outside this repository from bytes committed in the consumer's own tree

The platform SHALL provide a packaged, versioned distribution of
`@platform/design-tokens`, `@platform/react-ui` and `@platform/react-shell`
whose bytes are committed into the consuming repository. A consumer SHALL be
able to resolve the layer using only its own committed tree, its own native
package manager, and no network access, no sibling checkout, no
machine-specific absolute path and no environment variable. The distribution
mechanism SHALL be a `file:` dependency specifier declared in the consumer's
`package.json` that resolves to a committed package tarball beneath the
consumer's own repository root, and the committed tarball's bytes SHALL match
a digest recorded in this repository's feed manifest.

#### Scenario: A consumer resolves the layer with only its own committed tree

- **WHEN** a consumer repository declares
  `"@platform/design-tokens": "file:vendor/platform-ui/platform-design-tokens-0.1.0.tgz"`
  and the referenced tarball is committed at that repository-relative path
- **THEN** the consumer's own package manager resolves the package from that
  committed tarball
- **AND** resolution succeeds with no sibling checkout of this repository
  present, no environment variable set, and no network reachable

#### Scenario: The consumer tree is relocated before resolution

- **WHEN** a consumer tree containing the committed tarball is copied to an
  unrelated filesystem path and resolved there
- **THEN** resolution succeeds at the new path
- **AND** no absolute path recorded in the tree requires rewriting

#### Scenario: A dependency specifier names a machine-specific absolute path

- **WHEN** a consumer's dependency specifier for a platform UI package is an
  absolute filesystem path
- **THEN** the conformance check fails with a named `ui-feed-path-invalid`
  diagnostic that names the specifier
- **AND** the specifier is refused rather than resolved

#### Scenario: A dependency specifier escapes the consumer repository

- **WHEN** a consumer's dependency specifier for a platform UI package
  traverses outside the consumer repository root, including through a relative
  path containing `..`
- **THEN** the conformance check fails with a named `ui-feed-path-invalid`
  diagnostic that names the specifier
- **AND** the specifier is refused rather than resolved

#### Scenario: A dependency specifier is assembled from an environment variable

- **WHEN** a consumer's platform UI dependency path is produced by expanding an
  environment variable
- **THEN** the conformance check fails with a named `ui-feed-path-invalid`
  diagnostic naming the variable
- **AND** the check reports the same failure class it reports for an absolute
  path, because a variable a CI runner does not carry fails identically

#### Scenario: The feed is declared consumed but the tarball is absent

- **WHEN** a consumer declares the `consumes-feed` state and one required
  platform UI package has no committed tarball at its declared path
- **THEN** the check fails with a named `ui-feed-incomplete` diagnostic that
  names the missing package
- **AND** the consumer is not reported as conformant

#### Scenario: The committed tarball does not match the feed manifest

- **WHEN** a consumer's committed tarball digest differs from the digest
  recorded for that package in this repository's feed manifest, or from the
  digest the consumer itself declares
- **THEN** the check fails with a named `ui-feed-digest-mismatch` diagnostic
  that names the package and the digests that disagree
- **AND** the mismatch is not reported as a version difference

#### Scenario: The declared kit version differs from the feed manifest

- **WHEN** a consumer declares a `kitVersion` that differs from the version in
  this repository's feed manifest
- **THEN** the check fails with a named `ui-feed-version-mismatch` diagnostic
  naming both versions
- **AND** no package is resolved against the mismatched declaration

#### Scenario: The feed contains a package the consumer did not declare

- **WHEN** a consumer's committed feed directory contains a platform UI
  tarball that its declaration does not list
- **THEN** the check fails with a named `ui-feed-undeclared` diagnostic that
  names the undeclared package
- **AND** an undeclared package is treated as a defect rather than as an
  implicit upgrade

### Requirement: The platform SHALL emit every declared token, and SHALL emit no value that is not declared

`ui/tokens/tokens.json` SHALL be the single authoritative source of design
token values. The generator SHALL derive its output by walking that source
tree and resolving its alias references, and SHALL NOT enumerate token paths
individually. The generated CSS and TypeScript artifacts SHALL contain every
declared token value, SHALL NOT contain a literal color, space, radius or font
value that is not reachable from a declared value or a documented `var()`
alias, and SHALL NOT rename any token emitted by the previous version of the
generator.

#### Scenario: A token is added to the token source

- **WHEN** a token with a value and a type is added to `ui/tokens/tokens.json`
- **THEN** the generated CSS and TypeScript artifacts contain that token
- **AND** the generated artifact names the token in the platform's
  `--color-*`, `--space-*`, `--radius-*` or `--font-*` form derived from its
  source path

#### Scenario: A declared token is missing from the generated artifact

- **WHEN** the generated artifacts do not contain a token that the token source
  declares
- **THEN** verification fails with a named `ui-token-dropped` diagnostic that
  names the token's source path
- **AND** the omission is not reported as a passing build

#### Scenario: The generated artifact contains an undeclared literal

- **WHEN** the generated CSS contains a literal color, space, radius or font
  value that is not reachable from a declared value or a documented `var()`
  alias
- **THEN** verification fails with a named `ui-orphan-literal` diagnostic that
  names the declaration and its location
- **AND** a dark theme or any other mode carrying an undeclared literal fails
  the same way as a light-theme literal

#### Scenario: A token references an alias that does not resolve

- **WHEN** a declared token value references an alias that does not resolve to
  a declared value
- **THEN** verification fails with a named `ui-alias-unresolved` diagnostic
  that names the alias and the token that references it
- **AND** no partial or empty value is emitted for that token

#### Scenario: The generator is rewritten without a rename

- **WHEN** the generator is replaced by a tree walk rather than an enumerated
  template
- **THEN** the set of token names emitted is byte-equal to the set emitted by
  the previous generator
- **AND** a consumer pinned to the previous package version continues to
  resolve every token name it resolved before

### Requirement: The shared UI primitives SHALL include a canonical visually-hidden rule

The generated CSS SHALL define a single visually-hidden utility carrying the
full property set required to remove an element from visual presentation while
retaining it in the accessibility tree, and that definition SHALL NOT use
`!important`. The definition SHALL be emitted by the generator so that it
cannot drift from the token source, and no other rule in the generated
artifacts SHALL redefine it.

#### Scenario: The canonical rule is present and complete

- **WHEN** verification inspects the generated CSS
- **THEN** a visually-hidden definition is present
- **AND** it declares absolute positioning, a one-pixel box, zero padding, a
  negative one-pixel margin, hidden overflow, a zero-size clip, no wrapping
  white space, and zero border

#### Scenario: The canonical rule does not use !important

- **WHEN** verification inspects the visually-hidden definition
- **THEN** no declaration in that rule is marked `!important`
- **AND** the rule remains overridable by a consumer's own cascade

#### Scenario: The canonical rule is the only definition in the generated artifacts

- **WHEN** verification inspects every generated artifact
- **THEN** the visually-hidden selector is defined in exactly one of them
- **AND** a second, conflicting definition in another generated artifact fails
  verification

### Requirement: A repository SHALL be UI-conformant by not declaring a shared UI primitive itself, and a new divergent copy SHALL fail

The platform SHALL provide a workspace conformance check whose subject is the
absence of a second definition of a shared UI primitive, not the presence of
the shared definition, so that a repository shipping no user interface passes
without adopting anything. A repository that declares a shared visually-hidden
primitive in its own source SHALL fail the check, with the sole exception of
the repositories recorded in a frozen legacy baseline, and that baseline SHALL
NOT be extendable by any code path. A repository detected as shipping a user
interface SHALL be conformant only by declaring the `consumes-feed` state or
the `exempt` state with a reason, an owner and a review date.

#### Scenario: A repository declares its own visually-hidden rule

- **WHEN** a repository not recorded in the frozen legacy baseline declares a
  visually-hidden or screen-reader-only selector in its own source
- **THEN** the check fails with a named `ui-divergent-primitive` diagnostic
  that names the repository, the file, the line, and the canonical rule to use
  instead
- **AND** the repository is not reported as conformant

#### Scenario: A repository in the frozen legacy baseline is checked

- **WHEN** a repository recorded in the frozen legacy baseline is scanned
- **THEN** the check reports its existing definition as a known divergence
- **AND** the check does not fail solely because that definition exists
- **AND** the baseline entry is reported as blocking a new copy elsewhere

#### Scenario: The frozen legacy baseline has grown

- **WHEN** the frozen legacy baseline contains more entries than the count
  recorded when it was created
- **THEN** the architecture guard fails with a named `ui-legacy-list-grew`
  diagnostic
- **AND** no documented mechanism exists by which an entry could be added

#### Scenario: A repository ships a user interface but declares nothing

- **WHEN** a repository is detected as shipping a user interface and provides
  no conformance declaration
- **THEN** the check fails with a named `ui-conformance-undeclared` diagnostic
  that names the signal that detected the interface
- **AND** the repository is not reported as conformant by default

#### Scenario: A repository ships no user interface

- **WHEN** a repository is not detected as shipping a user interface and
  provides no conformance declaration
- **THEN** the check reports the repository as conformant
- **AND** the check does not require that repository to adopt the shared layer

#### Scenario: A repository declares the exempt state

- **WHEN** a repository ships a user interface and declares the `exempt` state
  with a reason, an owner and a review date, and declares no shared primitive
- **THEN** the check reports the repository as conformant
- **AND** the exemption and its reason are reported as visible in the check's
  output rather than silently passing

#### Scenario: A repository declares consumption of the feed

- **WHEN** a repository declares the `consumes-feed` state naming a feed path,
  a kit version and a digest per package, and every one of those agrees with
  the committed bytes and this repository's feed manifest
- **THEN** the check reports the repository as conformant
- **AND** the check does not require any other change in that repository

### Requirement: A new non-scale responsive value SHALL fail, and existing divergent values SHALL NOT block the change

The conformance check SHALL treat responsive breakpoints as a closed baseline
in the same manner as shared primitives. A repository SHALL NOT introduce a
hand-authored width-based media query that is not a documented kit scale value
unless it declares a reason, and the media queries that exist at the time this
capability is implemented SHALL be recorded in the frozen baseline and SHALL
NOT cause this check to fail.

#### Scenario: A new non-scale breakpoint is introduced

- **WHEN** a repository introduces a width-based media query value that is not
  a documented kit scale value and that is not in the frozen baseline
- **THEN** the check fails with a named `ui-breakpoint-undeclared` diagnostic
  that names the value and the file
- **AND** the check names the kit's scale-grounded value as the value to use

#### Scenario: A repository uses only baselined breakpoints

- **WHEN** a repository's width-based media query values are all present in the
  frozen baseline
- **THEN** the check does not fail on those values
- **AND** the repository is not required to have already unified its responsive
  scale for this capability to be conformant

#### Scenario: Two adjacent values that differ by one pixel

- **WHEN** a repository's baselined media query values include both a value
  and that value plus one pixel, at the same nominal breakpoint
- **THEN** the check reports the off-by-one pair as a known baseline divergence
- **AND** the check does not fail, because the pair predates this capability

### Requirement: This capability SHALL NOT add a .NET package and SHALL NOT alter the package manifest or the public API baseline

This capability SHALL be delivered without adding a .NET package, without
adding any `PackageReference` or `ProjectReference`, and without changing any
public .NET type. The generated package manifest SHALL remain byte-identical to
its state before this capability, the package inventory SHALL continue to
enumerate the same package count, and the public API baseline SHALL remain
unchanged. The conformance check SHALL be implemented as repository tooling and
as test methods in an existing architecture test project.

#### Scenario: The package manifest is unchanged

- **WHEN** the package manifest is regenerated and compared against the
  committed file
- **THEN** the manifest matches the source exactly
- **AND** the package count is the same as before this capability
- **AND** no new package appears in the inventory

#### Scenario: The public API baseline is unchanged

- **WHEN** the public API check runs and is compared against the committed
  baseline
- **THEN** no public type is added, changed or removed by this capability
- **AND** the check reports no difference

#### Scenario: The conformance check needs no new .NET package

- **WHEN** a consumer asks how to assert its own UI conformance
- **THEN** the answer is a committed feed plus a conformance declaration
  verified by repository tooling
- **AND** no assembly reference is required from the consumer, because the
  check's subject is the text of repositories rather than a runtime contract

#### Scenario: An existing consumer keeps resolving

- **WHEN** a consumer pinned to the previous version of a platform UI package
  installs it after this capability
- **THEN** every token name and utility it previously resolved still resolves
- **AND** no previously available declaration is removed

### Requirement: Razor consumers SHALL be served by the existing token artifact, and Flutter consumers SHALL NOT be claimed

This capability SHALL serve repositories whose user interface is delivered as
Razor Pages or other server-rendered HTML, by making the generated token CSS
available to them through the same committed feed, and SHALL NOT add a new
delivery vehicle for them. This capability SHALL NOT serve Flutter
consumers: it SHALL NOT produce a Dart token artifact, SHALL NOT add a Flutter
package, and SHALL NOT claim coverage of any Flutter repository.

#### Scenario: A Razor Pages repository consumes the token artifact

- **WHEN** a Razor Pages repository commits the generated token CSS into its
  own web assets and references it from its layout
- **THEN** its pages resolve the same token names as any other consumer
- **AND** no new .NET package, project or build integration is required for it
- **AND** its user interface remains subject to this capability's conformance
  check

#### Scenario: A Flutter repository is scanned

- **WHEN** the conformance check encounters a repository whose user interface
  is implemented in Dart
- **THEN** the check does not require that repository to adopt this
  capability
- **AND** this capability's coverage is not reported as including Flutter

#### Scenario: A Dart token artifact is requested

- **WHEN** a consumer asks for design tokens in Dart form
- **THEN** this capability does not provide them
- **AND** the request is directed to a separate change, because the platform
  ships no Dart token artifact and this capability does not invent one

### Requirement: The shipped token artifacts SHALL retain their existing accessibility guarantees

The generator rewrite SHALL NOT weaken any accessibility property the generated
artifacts currently assert. The generated CSS SHALL continue to provide a dark
theme, a reduced-motion fallback, a focus ring, and focus-visible styling, and
the shared components SHALL continue to expose busy, invalid and live status
markers, disabled behaviour, and a dialog role. Verification SHALL continue to
fail when any of these is absent.

#### Scenario: A dark theme is removed from the generated CSS

- **WHEN** the generated CSS no longer provides a dark theme selector
- **THEN** verification fails
- **AND** the change that removed it is reported as a regression rather than
  accepted as a simplification

#### Scenario: The reduced-motion fallback is removed

- **WHEN** the generated CSS no longer contains a reduced-motion fallback
- **THEN** verification fails

#### Scenario: An accessibility marker is removed from a shared component

- **WHEN** a shared component no longer exposes a busy, invalid or live status
  marker, its disabled behaviour, or its dialog role
- **THEN** verification fails and names the missing marker

#### Scenario: A consumer's own styles override the shared accessibility rule

- **WHEN** a consumer overrides a shared accessibility declaration in its own
  cascade
- **THEN** the consumer's declaration wins
- **AND** verification of the generated artifacts is unaffected, because it
  inspects this repository's artifacts rather than a consumer's overrides
