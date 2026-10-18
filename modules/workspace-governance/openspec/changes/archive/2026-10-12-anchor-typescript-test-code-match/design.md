# Design: Anchor the TypeScript test-code match on the qualifier

## 1. The measurement

Before changing anything, the false positives were counted by running the real
checker over the real repositories:

| Repository | `product-test-code` findings | What they were |
|---|---|---|
| `argoset` | 2 | `/^https?:\/\//.test(value)`; a doc comment naming `src/api/contract.test.ts` |
| `crossify` | 1 | `/attestation/i.test(err.message)` |
| `fotofy` | 6 | regex literals matched against a user-agent string |
| `transitfy` | 6 | `QUOTA_PATTERN.test(body)`, a scheme regex, a whitespace regex |

**15 findings, 0 real.** A rule that reports 15 false positives and no true ones
is not protecting anything; it is only making four repositories red.

## 2. Why a name match cannot work

`re.search(r"\b(describe|it|test)\s*\(", line)` asks one question — does this
line contain a call to something with this name — and answers it about a
qualifier it never looked at. The information needed to be correct is present in
the line and is discarded.

The discriminator is the character before the name:

| Line | Char before `test` | What it is |
|---|---|---|
| `test('x', ...)` | start, or whitespace | the framework global |
| `/re/.test(v)` | `.` | a regular expression |
| `PATTERN.test(v)` | `.` | a compiled `RegExp` |
| `"s".test(v)` | `.` | a `String` method |
| `re?.test(v)` | `.` | the same, optional-chained |
| `contest(1)` | a word character | a different identifier entirely |

So the fix is a lookbehind that rejects a preceding `.` or word character, plus
a pass that strips member calls before the final decision. Stripping first and
matching second is what makes it correct: a line like
`if (a.test(x) && test(y))` has both a member call and a bare call, and only
the second is a test.

## 3. Why comments are excluded separately

A doc comment naming `src/api/contract.test.ts` matched the pattern and was
reported. The remediation this rule exists to give — *move it into a test
file* — is nonsense for a comment, and a reviewer seeing that remediation on a
doc comment stops trusting the rule.

A comment prefix (`//`, `/*`, `*`) is a reliable signal in TypeScript, and a
comment cannot call anything, so it is excluded before matching. This is not a
general comment parser and does not need to be: it only has to avoid claiming
that prose is code.

## 4. What is deliberately not done

- **No exclude list of files or line numbers.** The same approach the Rust fix
  used, and rejected here for the same reason: an exclude list encodes today's
  false positives and silently accepts tomorrow's.
- **No language exemption.** The rule stays on for `typescript` and `javascript`.
- **No threshold.** Zero real findings in the rule's own terms is the goal, and
  it is reached by fixing the match, not by tolerating noise.

## 5. Verification

- The 4 repositories go from 15 findings to **0**, measured by running the real
  checker.
- Fleet-wide, `product-test-code` falls to **1**, and that one is a genuine
  `#[test]` in `alltools-platform`'s Rust source — not a TypeScript artefact.
- `TypeScriptQualifierTests` pins each measured false-positive line
  individually, and `test_a_bare_global_call_is_still_reported` is the negative
  control: if the anchor were dropped, those three would pass silently and the
  rule would be dead. That test failing is the signal.
