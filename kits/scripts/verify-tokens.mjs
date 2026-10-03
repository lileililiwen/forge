#!/usr/bin/env node
// Kit verifier for the `platform-ui-web` token source.
//
// Vendored from `dotnet-platform-libs/ui/scripts/verify-ui.mjs` at revision
// c740bd9d625f004978f058578e61ffcdf14bf8c2 and scoped to the artifacts a
// scaffold actually owns. The upstream verifier walks the whole library
// checkout; a generated project only owns the vendored token pair, so this
// variant asserts the same evidence over that pair and nothing else.
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

console.log('platform-ui-web token verification passed: vendored tokens carry the required design values.');
