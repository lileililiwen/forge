#!/usr/bin/env node
// Kit verifier for the `platform-ui-web` token source.
//
// Vendored from `dotnet-platform-libs/ui/scripts/verify-ui.mjs` at revision
// 6bf3946fcb1a9d2294b13d0af4347f6cf0d8e566 (kit 0.2.0) and scoped to the
// artifacts a scaffold actually owns. The upstream verifier walks the whole
// library checkout; a generated project only owns the vendored token pair, so
// this variant asserts the same evidence over that pair and nothing else: the
// pre-0.2.0 design-value checks, the adopted focus-ring alias
// (`--color-focus-ring`), and the 0.2.0 `tokens.ts` export shape
// (`tokenValues` and the light/dark `semanticColorValues` theme maps). The
// repo-wide completeness, template-package and Razor surfaces stay asserted by
// the upstream verifier in the kit repository.
//
// Runs offline with the project's own toolchain; it needs no registry
// dependency and no Forge runtime.

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// The tokens are resolved next to this script, so the same verifier works
// unchanged in the checked-in `kits/tokens/` tree and in a generated
// `.platform/tokens/` subtree.
const root = dirname(fileURLToPath(import.meta.url));
const cssPath = join(root, 'tokens.css');
const tsPath = join(root, 'tokens.ts');

const css = readFileSync(cssPath, 'utf8');
for (const token of [
  '--color-background',
  '--color-foreground',
  '--color-primary',
  '--color-danger',
  '--color-success',
  '--space-4',
  '--focus-ring',
  '--color-focus-ring',
]) {
  assert.match(css, new RegExp(`${token.replaceAll('-', '\\-')}\\s*:`), `missing token ${token}`);
}

// Theme and motion evidence the upstream verifier asserts.
assert.match(css, /\.theme-dark|\[data-theme="dark"\]/, 'missing dark theme');
assert.match(css, /prefers-reduced-motion/, 'missing reduced-motion fallback');

const ts = readFileSync(tsPath, 'utf8');
for (const token of ['color', 'background', 'primary', 'space']) {
  assert.match(ts, new RegExp(token), `missing TypeScript token export ${token}`);
}

// The 0.2.0 token module must carry the name -> value map and a complete
// light+dark semantic theme map, the exports framework theme adapters
// consume (upstream ui-design-guideline-adoption, D-5).
assert.match(
  ts,
  /export const tokenValues = \{[\s\S]*?"--color-background"[\s\S]*?\} as const;/,
  'missing tokenValues name -> value export',
);
const themeMap = /export const semanticColorValues = \{([\s\S]*?)\} as const;/.exec(ts);
assert.ok(themeMap, 'missing semanticColorValues theme export');
for (const mode of ['light', 'dark']) {
  assert.match(
    themeMap[1],
    new RegExp(`(?:^|[, ])${mode}: \\{[^{}]*background: "[^"]+"`),
    `semanticColorValues does not emit the ${mode} semantic theme`,
  );
}

console.log('platform-ui-web token verification passed: vendored tokens carry the required design values.');
