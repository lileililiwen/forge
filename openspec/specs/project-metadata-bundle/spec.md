# project-metadata-bundle Specification

## Purpose
Define a versioned, deterministic project metadata bundle from declared project sources, with receipt-protected writes and read-only search over registered projects.
## Requirements
### Requirement: Deterministic project metadata generation

Forge SHALL generate a `forge-project-metadata/1.0.0` document under `.forge/project-metadata.json` for an explicitly selected project, using only bounded files beneath that project root and declared metadata sources. It SHALL preserve project id; discoverability description, keywords, topics, audience, use cases and search phrases; declared capability ids, owners and evidence; complete interface ids, provide/consume side, provider, mechanism, contract, digest, entry, required and evidence fields; integration ids; documentation links; and canonical specification directory identifiers. Malformed declared metadata fields SHALL be refused, not silently omitted. The project README is documented as a source but its prose is not indexed. `source_revision` SHALL be a digest of the bounded declared source bytes and specification identifiers, not Git state. Forge SHALL NOT infer or fabricate missing values.

#### Scenario: Generate from declared project metadata

- **WHEN** a developer runs `forge project metadata build <PATH>` for a project with a valid `.project.json`
- **THEN** Forge writes a deterministic metadata document containing the declared discoverability, taxonomy, capability/interface, documentation and specification facts, with each list normalized, de-duplicated and sorted

#### Scenario: Regeneration does not change identical content

- **WHEN** the same inputs and source revision are built again
- **THEN** the artifact bytes and receipt remain byte-identical and the result is `unchanged`

#### Scenario: Missing or invalid source is refused

- **WHEN** `.project.json` is missing, malformed, has invalid required fields, or a bounded source file cannot be read
- **THEN** Forge refuses with the offending file/field and does not create or change either output file

### Requirement: Receipt-protected artifact ownership

Forge SHALL write only `.forge/project-metadata.json` and `.forge/project-metadata.receipt.json` in the explicitly selected project. Refresh SHALL require a valid receipt matching the current artifact digest; foreign, malformed, modified, or path-escaping targets SHALL be refused without overwriting user data.

#### Scenario: First generation creates a Forge-owned pair

- **WHEN** neither output exists and the selected project validates
- **THEN** Forge stages and promotes the document and receipt, which identifies the project, generator, relative path and SHA-256 digest; ordinary I/O failures restore the prior pair, while an interrupted promotion is diagnosed and never silently overwritten

#### Scenario: Edited generated artifact blocks refresh

- **WHEN** a user edits the generated document or receipt before a later build
- **THEN** Forge refuses with the conflicting path and leaves all bytes unchanged

#### Scenario: A generated artifact can be refreshed

- **WHEN** a valid receipt matches the current artifact and source metadata changes
- **THEN** Forge stages and promotes the document and receipt and reports `updated`; ordinary I/O failures restore the prior pair

### Requirement: Registered-project metadata search

Forge SHALL expose deterministic read-only search across generated metadata for projects in its existing local registry through the CLI and mature MCP surface. Search SHALL preserve the separate metadata contract, SHALL NOT alter the catalog contract, and SHALL report missing, malformed, incompatible, stale or unreadable bundles as per-project issues rather than treating them as empty matches.

#### Scenario: Agent searches registered projects

- **WHEN** a caller supplies a valid query to `search_project_metadata` or `forge project metadata search`
- **THEN** Forge returns matching project ids, the matched declared metadata fields, source revision and the metadata contract version, with deterministic relevance and tie ordering

#### Scenario: Query tokens and ranking are deterministic

- **WHEN** a query contains multiple whitespace-separated terms
- **THEN** every term must match at least one indexed metadata field; results rank by matched field count descending, then project id and path ascending

#### Scenario: One project bundle is unavailable

- **WHEN** a registered project has no valid bundle while another project has a valid matching bundle
- **THEN** the valid result is returned and the missing/invalid project is named in `issues`; it is not silently counted as a no-match

#### Scenario: No metadata matches

- **WHEN** every readable bundle is valid but none matches the query
- **THEN** Forge returns an explicit empty `matches` list and any independent per-project issues

#### Scenario: Search changes nothing

- **WHEN** a CLI or MCP metadata search runs
- **THEN** project files and the local registry remain byte-identical and Forge does not discover parent paths, workspace roots, remote sources, or CodeGraph indexes

#### Scenario: Invalid query is refused

- **WHEN** query is empty, exceeds 128 characters, has more than eight terms, or limit is outside 1..=50
- **THEN** Forge returns a typed invalid-argument error before reading registered projects

