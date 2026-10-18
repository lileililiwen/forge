# product-code-quality-boundary Specification (delta: anchor-typescript-test-code-match)

## MODIFIED Requirements

### Requirement: Reject test code in product scope

The quality checker SHALL report a test call found in product scope as
`product-test-code` at ERROR, and SHALL NOT report a call that is not a test.

For TypeScript and JavaScript, the checker SHALL match `describe`, `it` and
`test` only when the call is made on a **bare identifier** — the framework
global. A call on any expression is a method on that expression, and a method
named `test` is a method: a regular expression literal, a compiled `RegExp`, or
a `String` method are all product code. A comment line SHALL NOT be matched at
all, because a comment cannot call anything.

#### Scenario: Rust cfg test module

- **WHEN** a Rust product file contains a test module under `#[cfg(test)]`
- **THEN** the test module is allowed and produces no boundary finding

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

- **WHEN** the qualifier anchor were removed to silence the cases above
- **THEN** the bare-call scenarios above would be reported as clean, and a test
      asserting they are still reported would fail
