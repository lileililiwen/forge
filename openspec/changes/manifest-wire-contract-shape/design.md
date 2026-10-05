# Design: The public manifest is produced in the shape the contract defines

## 1. Implementation boundary

Repository `/home/paul/code/forge`, Rust. Three source files change:

- `src/portfolio/share/mod.rs` — the two pinned constants.
- `src/portfolio/share/manifest.rs` — the two serialized field types, the
  revision encoding, and the in-module tests.
- `src/main.rs`, `src/api/mod.rs` — follow the type; no logic change.

Tests: `tests/portfolio_share_cli_contract.rs` (the two stale assertions) and a
new `tests/manifest_wire_contract.rs`.

Not changed: `src/registry/share/**`, `src/portfolio/publication.rs`,
`src/portfolio/share/publish.rs`, `src/portfolio/share/audit_types.rs`,
`contracts/**`, `scripts/contract-parity.sh`.

`/home/paul/code/platform-contracts/` is read-only. The acceptance test reads
one file from it and writes nothing.

## 2. Ownership of each field

The contract owns the wire shape. Forge owns the *content* of the fields and
nothing else, so each pin is a single constant with no per-call-site freedom.

**`schema_family`.** A closed enum with one member upstream, so it is a
constant: `platform.public-portfolio-manifest`. The old value was the contract's
file *stem* rather than its family name. Nothing in `src/` cross-references the
old value — `grep -rn "portfolio-manifest" src/` matches only this constant and
an unrelated test path in `publish.rs` — so the change has no second consumer.

**`schema_version`.** The contract requires a full semantic version
(`^[0-9]+\.[0-9]+\.[0-9]+$`), not a major. Forge supports major 1 and has never
produced a minor or patch variant, so the version it produces is the literal
`"1.0.0"`. The constant becomes `&str`. Keeping a separate major constant would
add an indirection with exactly one value and no caller that could disagree;
when a `1.1.0` contract appears the version string is the thing that changes
anyway.

**`manifest_revision`.** The contract calls it a "stable producer-side revision
identifier (for example, a content hash or a sequence tag)" — an opaque string,
`1..64` characters, over `[A-Za-z0-9._:-]` after a leading alphanumeric.

Forge's internal revision is a monotonically increasing integer, allocated by
the registry and already the approval's identity. The encoding is therefore a
**pure, injective, total function of that integer**:

```rust
pub fn wire_manifest_revision(revision: u32) -> String {
    format!("rev_{revision}")
}
```

*Deterministic* — the same integer always yields the same string, with no
clock, no counter and no random component.
*Schema-conformant* — `r` is alphanumeric and `e`, `v`, `_`, and the decimal
digits are all inside `[A-Za-z0-9._:-]`. `rev_` plus at most ten `u32` digits is
14 characters, well inside the 64 ceiling. This is asserted against the real
pattern in a test rather than assumed.
*Distinguishable from a content hash* — the `rev_` prefix means a consumer
reading the field can tell a sequence tag from a digest, which is exactly the
distinction the contract's description draws.
*Injective* — no two revisions collide, so the approval identity survives the
encoding.

The encoding lives in one function so no call site can invent its own spelling.
`build_manifest` keeps its `u32` parameter: the boundary that changes types is
the struct field, not the domain API.

## 3. Why the field type changes rather than a serde shim

`ManifestBody` is the canonical, hashed body. A `#[serde(serialize_with = …)]`
attribute would leave the in-memory field a `u32` while writing a string —
which reads as though the integer *is* the wire value, and leaves the
deserialized type and the in-memory type disagreeing. Both the body and the
published document are the contract's document, so their field types are the
contract's field types. `Serialize` and `Deserialize` stay paired and the
round-trip in `the_document_embeds_the_hash_and_the_emission_time` keeps
asserting something true.

## 4. Determinism and the hash

`ManifestBody::sha256()` hashes `canonical_json()`. The body carries the
contract's fields only; `generated_at` and `manifest_sha256` are deliberately
outside it, and that design is untouched.

An unchanged catalog still hashes identically across runs and platforms: the
records sort the same way, and `rev_<revision>` is a pure function of an integer
that the records did not change. What does change is the **hash value** for any
given catalog, because a field in the hashed body changed shape. That has one
real consequence, stated plainly:

> An approval made by a build before this change is bound to the old
> `manifest_sha256`. After the upgrade `publish_approved_manifest` recomputes
> the current hash, finds the mismatch and refuses with
> `PortfolioShareConflict` — "the share records changed after revision N was
> approved". The operator's remedy is the documented one: preview and approve
> the revision again. No approval row, publication row or migration is touched,
> and no historical audit entry is rewritten.

This is accepted rather than worked around. A migration that rewrote the stored
hash would forge an approval that was never given, which is the one thing the
approval gate exists to prevent.

## 5. Preview and API surfaces

`forge portfolio share preview` (human and JSON) and `GET /v1/share/manifest`
both echo `draft.body.manifest_revision`. That is the *manifest* revision, and
it now reads back in its wire form, which is what an operator comparing the
preview against the published document needs to see. Neither surface performs
arithmetic on it, so no logic changes.

The surfaces that *do* treat the revision as a number — `PublishReport`,
`PublishContext`, `AdapterRequest`, `PublicationAttempt`, the audit query in
`src/registry/share/audit.rs` and the `manifest_revision INTEGER` column — read
`approval.revision` or the stored row, never `draft.body.manifest_revision`, so
all of them keep the integer type and none of them changes.

## 6. Verification design

Two levels, because they fail differently.

1. **In-repo pinning.** `the_document_matches_the_contract_shape` asserts the
   exact field values and matches them against the schema's own patterns and
   enum, so a future edit to either constant fails a Rust test with no sibling
   checkout present. `manifest_revision_encoding_is_stable_and_schema_shaped`
   covers the encoding function over the boundary values, including `u32::MAX`.
2. **The decisive check.** `tests/manifest_wire_contract.rs` builds a manifest
   through the production `build_manifest` / `ManifestDraft::document` path and
   validates the serialized bytes against the sibling
   `public-portfolio-manifest.schema.json` with `jsonschema`. This is the
   consumer's schema, applied to a document Forge actually produced.

The second test needs the sibling checkout. It is `#[ignore]`d with a printed
note when `PLATFORM_CONTRACTS_DIR` / `../platform-contracts` does not resolve,
and reports `unverified` rather than a pass — the same rule the repository
already applies to
`generate_contract.rs::dotnet_scaffold_builds_offline_without_forge`. A test
that silently skips its own oracle is the "green result that never ran the
check" defect this repository has already fixed once.

The three non-goal mismatches in the proposal are the reason the acceptance
fixture is chosen, not hidden: it uses a record whose `visibility` is `public`,
whose `status_evidence` is absent, and whose `id` is inside the contract's
64-character ceiling, so the document validates. Those three gaps stay open and
reported.
