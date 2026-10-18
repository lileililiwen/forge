## MODIFIED Requirements

### Requirement: Reject test code in product scope

The checker MUST report a blocking finding when test-only code is found inside
product scope, except for Rust test code that the crate's own module tree says
is compiled only under `#[cfg(test)]`; repository test roots MUST remain
outside product scope.

For Rust, whether a file is test code MUST be decided from the module tree
and not from the file's own contents alone. A Rust file inside a product root
MUST be treated as test scope when either signal says so:

1. the item in the file is lexically enclosed by a `#[cfg(test)]` attribute in
   that same file, or
2. the crate declares the file's module under `#[cfg(test)]` — a sibling
   `mod.rs`, `lib.rs` or `main.rs` in the file's own directory, or the parent
   directory's `mod.rs` for a directory module, carries `#[cfg(test)]` on the
   `mod <name>;` declaration that reaches the file.

A `#[cfg(test)]` guard MUST attach to the item that follows it, so
intervening attribute lines (`#[path = "tests.rs"]`, `#[allow(...)]`) MUST NOT
disarm it. An ambiguous guard — `#[cfg(any(test, …))]`, `#[cfg(all(test, …))]`
— MUST NOT authorize test scope, exactly as before.

The checker MUST NOT decide this from a file-name list. `tests.rs`,
`*_test.rs` and `test_*.rs` are the conventional names for a Rust test module
and MUST NOT on their own exempt a file: a file with one of those names and no
`#[cfg(test)]` declaration reaching it is still product test code and MUST
still be reported. No `exclude` entry, skip or filename exception list may be
added instead of reading the declaration, because a check that reports a
healthy repository as broken is worse than no check, and an exclusion hides a
real finding rather than reporting one.

#### Scenario: Rust cfg test module

- **WHEN** a Rust product file contains a test module under `#[cfg(test)]`
- **THEN** the test module is allowed and produces no boundary finding

#### Scenario: A cfg test module declared in another file

- **WHEN** a Rust product file `tests.rs` sits beside a `mod.rs` that declares
  `#[cfg(test)] mod tests;`
- **THEN** the file is test scope and produces no `product-test-code` finding

#### Scenario: The declaration is what authorizes, not the name

- **WHEN** a Rust product file is named `tests.rs`, sits beside no `mod.rs`
  that declares it under `#[cfg(test)]`, and contains `#[test]` functions
- **THEN** the checker reports `product-test-code` as `FAIL`

#### Scenario: An intervening attribute does not disarm the guard

- **WHEN** a Rust product file declares `#[cfg(test)]`, then `#[path = "tests.rs"]`,
  then `mod tests;`
- **THEN** neither the declaration nor the test functions behind it are
  reported

#### Scenario: Rust integration code in source

- **WHEN** a Rust product file contains an integration-test module or test-only
  imports outside `#[cfg(test)]`
- **THEN** the checker reports `product-test-code` as `FAIL`

#### Scenario: Non-Rust test module

- **WHEN** a Python, TypeScript, C#, or supported product file contains a
  recognized test-only module/file pattern
- **THEN** the checker reports `product-test-code` as `FAIL`

#### Scenario: A framework global in product scope

- **WHEN** a `.ts` file in product scope contains `describe('x', () => {})`,
  `it('y', () => {})` or `test('z', () => {})` on a bare identifier
- **THEN** the checker reports one `product-test-code` finding at ERROR

#### Scenario: A regular expression is not a test

- **WHEN** a line contains `/pattern/.test(value)` or `PATTERN.test(value)`
- **THEN** the checker reports nothing

#### Scenario: A String method is not a test

- **WHEN** a line contains `"text".test(value)`
- **THEN** the checker reports nothing

#### Scenario: An optional-chained member call is not a test

- **WHEN** a line contains `re?.test(body)`
- **THEN** the checker reports nothing

#### Scenario: Prose about a test file is not a test

- **WHEN** a comment line names a `*.test.ts` file
- **THEN** the checker reports nothing

#### Scenario: A longer identifier is not a test

- **WHEN** a line contains `contest(1)` or `const latest = compute()`
- **THEN** the checker reports nothing, because the identifier is not `test`

#### Scenario: The rule is still live

- **WHEN** the qualifier anchor were removed to silence the TypeScript cases
- **THEN** the bare-call test asserting the global is reported would fail
