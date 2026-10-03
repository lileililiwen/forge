// Self-test for ui/scripts/verify-ui.mjs.
//
// Every named diagnostic the verifier can emit is proved to fire, by mutating a
// throwaway copy of the tree and asserting the failure. A verifier whose own
// failure paths are unexercised is the `contract-parity: OK` failure class this
// change exists to prevent (design.md D18), so the negative cases run in the
// quality gate alongside the positive one.
//
// Usage: node ui/scripts/selftest-verify-ui.mjs

import assert from 'node:assert/strict';
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verify } from './verify-ui.mjs';

const uiRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const repositoryRoot = join(uiRoot, '..');

const ARTIFACTS = [
  'ui/tokens/tokens.json',
  'ui/tokens/emitted-baseline.json',
  'ui/packages/design-tokens/dist/tokens.css',
  'ui/packages/design-tokens/dist/tokens.ts',
  'ui/packages/react-ui/src/components.tsx',
  'ui/packages/react-ui/styles.css',
  'ui/packages/react-shell/src/shell.tsx',
  'src/Platform.UI.Razor/wwwroot/platform-tokens.css',
  'src/Platform.UI.Razor/wwwroot/platform-ui.css',
];

function fixtureRoot() {
  const root = mkdtempSync(join(tmpdir(), 'platform-ui-selftest-'));
  for (const relative of ARTIFACTS) {
    const target = join(root, relative);
    cpSync(join(repositoryRoot, relative), target, { recursive: true });
  }
  return root;
}

function edit(root, relative, mutate) {
  const path = join(root, relative);
  const before = readFileSync(path, 'utf8');
  const after = mutate(before);
  assert.notEqual(after, before, `self-test mutation for ${relative} changed nothing`);
  writeFileSync(path, after);
}

const cases = [
  {
    name: 'ui-token-dropped',
    expected: /ui-token-dropped.*color\.slate\.500/s,
    apply: (root) => edit(root, 'ui/packages/design-tokens/dist/tokens.css', (css) => css.replace('  --color-slate-500: #64748b;\n', '')),
  },
  {
    name: 'ui-token-dropped in tokens.ts',
    expected: /ui-token-dropped.*tokens\.ts/s,
    apply: (root) => edit(root, 'ui/packages/design-tokens/dist/tokens.ts', (ts) => ts.replace('"--space-4": "1rem", ', '')),
  },
  {
    name: 'ui-undeclared-token',
    expected: /ui-undeclared-token.*--color-invented/s,
    apply: (root) => edit(root, 'ui/packages/design-tokens/dist/tokens.css', (css) => css.replace(':root {', ':root {\n  --color-invented: #abcdef;')),
  },
  {
    name: 'ui-orphan-literal in the dark theme',
    expected: /ui-orphan-literal.*#123456/s,
    apply: (root) => edit(root, 'ui/packages/design-tokens/dist/tokens.css', (css) => css.replace('[data-theme="dark"], .theme-dark {\n  --color-background: var(--color-slate-900);', '[data-theme="dark"], .theme-dark {\n  --color-background: #123456;')),
  },
  {
    name: 'ui-alias-unresolved in a token',
    expected: /ui-alias-unresolved.*color\.red\.999/s,
    apply: (root) => edit(root, 'ui/tokens/tokens.json', (json) => json.replace('"danger": { "$value": "{color.red.700}"', '"danger": { "$value": "{color.red.999}"')),
  },
  {
    name: 'ui-alias-unresolved in a theme override',
    expected: /ui-alias-unresolved.*theme\.dark\.color\.primary/s,
    apply: (root) => edit(root, 'ui/tokens/tokens.json', (json) => json.replace('"primary": "{color.blue.300}"', '"primary": "{color.blue.999}"')),
  },
  {
    name: 'ui-token-renamed (V6: a 0.1.0 name may not change value)',
    expected: /ui-token-renamed.*--space-4/s,
    apply: (root) => edit(root, 'ui/packages/design-tokens/dist/tokens.css', (css) => css.replace('--space-4: 1rem;', '--space-4: 1.1rem;')),
  },
  {
    name: 'ui-primitive-incomplete (a property is missing)',
    expected: /ui-primitive-incomplete.*margin/s,
    apply: (root) => edit(root, 'ui/packages/design-tokens/dist/tokens.css', (css) => css.replace('  margin: -1px;\n', '')),
  },
  {
    name: 'ui-primitive-incomplete (!important is refused)',
    expected: /ui-primitive-incomplete.*!important/s,
    apply: (root) => edit(root, 'ui/packages/design-tokens/dist/tokens.css', (css) => css.replace('  border: 0;\n}', '  border: 0 !important;\n}')),
  },
  {
    name: 'ui-duplicate-primitive',
    expected: /ui-duplicate-primitive.*platform-ui\.css/s,
    apply: (root) => edit(root, 'src/Platform.UI.Razor/wwwroot/platform-ui.css', (css) => `${css}\n.visually-hidden { position: absolute; }\n`),
  },
  {
    name: 'pre-existing gate: dark theme removed',
    expected: /missing dark theme/,
    apply: (root) => edit(root, 'ui/packages/design-tokens/dist/tokens.css', (css) => css.replaceAll('[data-theme="dark"], .theme-dark', '.theme-light')),
  },
  {
    name: 'pre-existing gate: reduced-motion removed',
    expected: /missing reduced-motion fallback/,
    apply: (root) => edit(root, 'ui/packages/design-tokens/dist/tokens.css', (css) => css.replace('prefers-reduced-motion', 'no-preference')),
  },
  {
    name: 'pre-existing gate: accessibility marker removed',
    expected: /missing accessibility marker aria-live/,
    apply: (root) => edit(root, 'ui/packages/react-ui/src/components.tsx', (source) => source.replaceAll('aria-live', 'aria-polite')),
  },
  {
    name: 'pre-existing gate: focus-visible removed',
    expected: /missing focus-visible styling/,
    apply: (root) => edit(root, 'ui/packages/react-ui/styles.css', (css) => css.replaceAll(':focus-visible', ':focus')),
  },
  {
    name: 'pre-existing gate: required artifact removed',
    expected: /missing ui\/packages\/react-shell\/src\/shell\.tsx/,
    apply: (root) => rmSync(join(root, 'ui/packages/react-shell/src/shell.tsx')),
  },
];

const roots = [];
try {
  const pristine = fixtureRoot();
  roots.push(pristine);
  verify(pristine);
  console.log(`  ok  positive control: the unmodified tree passes (${cases.length + 1} cases)`);

  for (const testCase of cases) {
    const root = fixtureRoot();
    roots.push(root);
    testCase.apply(root);
    assert.throws(() => verify(root), testCase.expected, `expected ${testCase.name} to fail with ${testCase.expected}`);
    console.log(`  ok  ${testCase.name}`);
  }
} finally {
  for (const root of roots) rmSync(root, { recursive: true, force: true });
}

console.log(`verify-ui self-test passed: 1 positive control and ${cases.length} negative cases each produced their named diagnostic.`);
