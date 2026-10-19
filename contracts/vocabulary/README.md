# Consumed vocabulary: which list governs which concern

Forge maintains and consumes these word lists. Each list has one Forge owner
and a digest in `../manifest.json`.

| File | Source | Governs |
| --- | --- | --- |
| `governance-vocabulary.json` | Forge's bundled Workspace Governance vocabulary | Declaration values: `kind`, `profile`, repository-check words (`placeholder_markers`) |
| `secret-field-substrings.json` | Forge's contract redaction vocabulary | Contract field names in shared documents |

Role split, enforced by the consumer:

- **Contract field names in shared documents** come from the maintained
  `secret-field-substrings.json` list. They name fields, never values.
- **Repository-check words** (what the product-code quality checker
  counts) come from Forge's bundled vocabulary
  (`placeholder_markers`). A project may narrow them with an explicit
  `placeholder_markers` override in its `.project.json` `quality`
  block, with the reason recorded in `.ai-rules/completion.md` — never
  by raising the threshold.
- **Captured-output redaction** stays in `policy::redact_credentials`
  (`src/policy/mod.rs`). It matches credential *shapes* (token formats,
  `key=value` secrets), not word lists, and is a separate concern from
  either list above. Nothing here replaces it.

Absence rule: if `governance-vocabulary.json` is missing or fails its
manifest digest, every consulting surface behaves as it did before this
vocabulary existed and reports `vocabulary-unavailable`. Absence never
blocks generation, import, doctor, check, fleet or gate, and no value
is ever fetched from the network or guessed locally as a substitute.
