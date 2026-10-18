# Design: Product-code quality gates

## Ownership and boundaries

Workspace Governance owns the portable checker and its finding contract.
Projects own source-root declarations, language profiles, exclusions, and
legacy thresholds. Driftwatchdog owns execution and aggregation, not parsing
project languages. Forge consumes normalized observations and is unchanged.

## Policy input

The checker reads a project declaration section:

```json
{
  "quality": {
    "product_roots": ["src"],
    "languages": ["rust"],
    "exclude": ["src/generated"],
    "placeholder_threshold": 0
  }
}
```

`product_roots` are required for an applicable check. `languages` selects
language rules; unknown language names are configuration errors. Exclusions
must be below a declared root and are reported in the result. A threshold is
an integer count across product files; changed-scope mode may apply the same
rule to changed files while the full gate remains authoritative.

## Boundary rules

The scanner classifies paths before content. Test roots and generated/vendor/
build paths are outside product scope. Within product scope, Rust test modules
are allowed only when their module body is dominated by an enclosing
`#[cfg(test)]` item. Rust `#[cfg(any(test, ...))]` and non-test cfgs do not
authorize test-only code. Other languages use conservative file/module
patterns for common test names and test-framework declarations; ambiguous
matches produce `REVIEW_REQUIRED`, not a guessed pass.

The checker never treats documentation outside product roots as product code.
Comments inside product files are scanned for placeholder debt because they are
maintainer-visible work markers; markers in the checker itself and test
fixtures are excluded by path policy.

## Placeholder rules and findings

Markers are case-insensitive and word/lexeme bounded: `TODO`, `FIXME`, `XXX`,
`HACK`, `placeholder`, `stub`, `unimplemented!`, and `todo!`. Each finding
contains `rule_id`, relative path, line, marker, severity, and remediation.
The report includes counts by marker and by file. Threshold comparison occurs
after exclusions and fails closed if any product file cannot be read.

## Output and integration

The command supports human output by default and `--json` for one stable
versioned document. Exit 0 means no blocking finding; exit 1 means a quality
failure; exit 2 means invalid or incomplete policy/configuration. The existing
Workspace Governance adapter maps these statuses to its current observation
vocabulary. Driftwatchdog project manifests bind the command as a normal Gate
check, so no language-specific evaluator enters the Gate core.

## Migration

New generated declarations use `placeholder_threshold: 0`. Existing projects
receive an explicit baseline threshold only after an audit records the current
count; the baseline is temporary policy data, not a pass claim. Each migrated
project must reduce the baseline to zero or document a narrowly scoped,
reviewed exception. Missing migration metadata stays `REVIEW_REQUIRED`.

## Verification

Fixtures prove allowed Rust `#[cfg(test)]`, forbidden product integration/test
code, non-Rust test leakage, marker thresholds, exclusions, unreadable files,
JSON stability, and secret redaction. Run the Workspace Governance unit suite,
self-check, and the project Gate command. Structural OpenSpec validation is
necessary but does not prove every portfolio project has been migrated.
