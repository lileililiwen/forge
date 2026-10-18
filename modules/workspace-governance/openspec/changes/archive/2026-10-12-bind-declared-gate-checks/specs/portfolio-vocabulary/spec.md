## MODIFIED Requirements

### Requirement: Shared word lists have one source

The secret scan MUST use `secret_field_substrings` from `vocabulary.json`, and
the product-code quality checker MUST default its placeholder markers from
`placeholder_markers`. A per-project `placeholder_markers` override MUST still
be honoured. The secret list MUST remain a superset of the previous terms.

**The marker vocabulary is restricted to unambiguous code-debt tokens.** A word
that is ordinary English MUST NOT be in `placeholder_markers`, because
`product_code_quality.py` matches case-insensitively on word boundaries across
every product line: prose that *describes* the word, a doc comment naming it,
and the gate's own `placeholder-threshold` concern identifier all match it. A
check that fires on the English language produces findings that are not debt,
and a check that cries wolf is ignored — which is strictly worse than a check
that does not run.

`placeholder` and `stub` were removed on 2026-10-05 for exactly this reason.
Measured over `driftwatchdog/src/`: the word `placeholder` matched **26** times
and **zero** of the 26 was unfinished work. Every one was either prose
describing the word (`//! ... extends this placeholder`, `/// Ordered,
comma-separated placeholder names`) or the concern identifier
`placeholder-threshold` in `src/gate/adapters.rs`, against 1 `TODO` and 1
`FIXME` on the same tree. The retained set is `todo`, `fixme`, `xxx`, `hack`,
`unimplemented!`, `todo!`.

**This reduces the finding count, and that is stated rather than hidden.**
Across the 39 repositories the derived `quality` block was written to, the
marker findings fall from **468 to 38**, and the repositories reporting
`FAIL` fall from **28 to 13**. 430 of the 468 were the two ordinary English
words. The number is reported because a vocabulary edit that only ever lowers a
count is indistinguishable from a filter; the difference is that this edit
corrects a mis-specified vocabulary in the one place that owns it, changes no
threshold, adds no `exclude`, skips no test, and leaves the finding machinery
exactly as it was.

**This is a correction, not a filter, and the distinction is load-bearing.** An
`exclude` names files the checker must not look at; a threshold buys a
tolerance; a `skip` hides a test. None of those is what happened. The
vocabulary answers a narrower question — *which words mean unfinished work* —
and it now answers it correctly. Every `TODO` and `FIXME` in the fleet is
still reported, and `product_code_quality.py` is unchanged.

A hand-written per-project `placeholder_markers` list MUST be left exactly as
its author wrote it. A project that wants a stricter list than the portfolio
default has said so on purpose, and rewriting a deliberate local decision to
match a portfolio default is a bigger error than the noise it removes. Only
`forge` carries such a list in this fleet, and it already names the six
retained tokens, so in practice the override and the default agree there.

#### Scenario: Marker default from vocabulary

- **WHEN** a project declares no `placeholder_markers`
- **THEN** the checker uses the vocabulary's list

#### Scenario: Override still honoured

- **WHEN** a project declares its own `placeholder_markers`
- **THEN** the checker uses those instead, unchanged by any portfolio edit

#### Scenario: An ordinary English word is not a finding

- **WHEN** a product line contains the word `placeholder` or `stub`, in prose,
  in a doc comment, or inside an identifier such as `placeholder-threshold`
- **THEN** the checker reports no `placeholder-marker` finding for it

#### Scenario: A real debt token is still a finding

- **WHEN** a product line contains `TODO` or `FIXME`
- **THEN** the checker reports a `placeholder-marker` finding for it, at any
  threshold, and the count still drives the `placeholder-threshold` verdict

#### Scenario: Secret scan still fires

- **WHEN** a registry entry contains a value under a secret-shaped key named in
  the vocabulary
- **THEN** the audit reports ERROR `SECRET_VALUE`
