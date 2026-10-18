# Proposal: Anchor the TypeScript test-code match on the qualifier

## Why

`product_code_quality.py` reports a `describe(`/`it(`/`test(` call in product
scope as test code. The match was lexical, so it fired on anything with that
spelling. Measured across the fleet: **15 findings in 4 repositories, and every
one was a false positive.**

| Repository | Findings | What they actually were |
|---|---|---|
| `argoset` | 2 | a regex literal; a doc comment naming a `*.test.ts` file |
| `crossify` | 1 | `/attestation/i.test(err.message)` |
| `fotofy` | 6 | regex literals matched against a user-agent string |
| `transitfy` | 6 | compiled-regex `.test()` and a string method |

This is the same class of defect the Rust scanner had, and it was fixed there
by reading the module tree instead of the file name. TypeScript has no
equivalent module-tree signal, but it has something better and simpler: **the
qualifier**.

```ts
describe('thing', () => {})              // a call on a bare identifier — the framework global
if (/attestation/i.test(err.message))     // a call on an expression — a regex
} else if (QUOTA_PATTERN.test(body))      // ditto — a compiled RegExp
return "...".test(value);                 // ditto — a String method
```

A method named `test` is a method, whatever it is called on. Only a call on a
bare identifier is the framework global. The whole defect is that the rule
matched the name and ignored the qualifier.

## What changes

One scanner, one function. `describe`/`it`/`test` are matched only when they are
a **bare call**, and a line that is a comment is not a call at all — prose about
tests is not test code, and the remediation this rule exists to give ("move it
into a test file") makes no sense for a doc comment.

## What does not change

The rule still fails a real test in product scope. `test_describe_call_in_source_is_rejected`
already pinned that and still passes. A new test pins each of the measured
false-positive lines individually, and a matching negative control pins that
`describe(`, `it(` and `test(` on a bare identifier are **still** reported — if
the anchor were dropped to silence the false positives, that control goes red
rather than the rule quietly dying.

No exclude list. No filename list. No language turned off. The same approach
the Rust fix used.
