// Verify the shipped UI token layer.
//
// Every assertion that existed before the DTCG tree walk (design.md V5) is still
// here, unchanged: a regression in this file's older checks is a regression of
// that change, not a simplification. On top of them it now also proves the three
// properties the old verifier could not see:
//
//   * completeness   - every $value in tokens.json reaches tokens.css/tokens.ts
//                      (ui-token-dropped); the 0.1.0 names are never dropped or
//                      renamed (ui-token-renamed, the V6 invariant)
//   * no orphan value - no literal in the generated CSS that is not reachable
//                      from a $value or a documented var() alias
//                      (ui-orphan-literal) - this is what closes the dark theme
//                      carrying a second literal palette
//   * canonical primitive - .visually-hidden exists once, with all nine
//                      properties and no !important (ui-duplicate-primitive,
//                      ui-primitive-incomplete)
//
// Usage: node ui/scripts/verify-ui.mjs

import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import {
  CANONICAL_VISUALLY_HIDDEN,
  TokenCheckError,
  declaredLiterals,
  parseCustomProperties,
  readThemeOverrides,
  readTokens,
  resolveAliases,
} from './lib/token-model.mjs';

const uiRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const repositoryRoot = join(uiRoot, '..');

/** Unchanged from 0.1.0 (design.md V5: the required-artifact list is a gate, not a suggestion). */
export const REQUIRED_ARTIFACTS = [
  'ui/tokens/tokens.json',
  'ui/packages/design-tokens/dist/tokens.css',
  'ui/packages/design-tokens/dist/tokens.ts',
  'ui/packages/react-ui/src/components.tsx',
  'ui/packages/react-shell/src/shell.tsx',
  'ui/packages/react-templates/package.json',
  'ui/packages/react-templates/src/index.ts',
  'ui/packages/react-templates/src/layouts.tsx',
  'ui/packages/react-templates/src/theme.tsx',
  'ui/packages/react-templates/styles.css',
  'src/Platform.UI.Razor/wwwroot/platform-ui.css',
];

/** Every generated or shipped stylesheet that may define the canonical primitive. */
const PRIMITIVE_BEARING_ARTIFACTS = [
  'ui/packages/design-tokens/dist/tokens.css',
  'ui/packages/react-ui/styles.css',
  'src/Platform.UI.Razor/wwwroot/platform-tokens.css',
  'src/Platform.UI.Razor/wwwroot/platform-ui.css',
];

const PRIMITIVE_BEARING_MODULES = [
  'ui/packages/design-tokens/dist/tokens.ts',
  'ui/packages/react-ui/src/components.tsx',
  'ui/packages/react-shell/src/shell.tsx',
];

function read(root, relative) {
  return readFileSync(join(root, relative), 'utf8');
}

/** Body of the first `selector { ... }` rule in `css`, or undefined. */
function ruleBody(css, selector) {
  const start = css.search(new RegExp(`(^|[\\s,>+~])${selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}(?![\\w-])[^{}]*\\{`, 'm'));
  if (start < 0) return undefined;
  const open = css.indexOf('{', start);
  const close = css.indexOf('}', open);
  return close < 0 ? undefined : css.slice(open + 1, close);
}

/** Declarations inside a rule body, in order, as [property, value] pairs. */
function declarations(body) {
  return body
    .split(';')
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => {
      const index = part.indexOf(':');
      return [part.slice(0, index).trim(), part.slice(index + 1).trim()];
    });
}

function fail(code, message) {
  throw new TokenCheckError(code, message);
}

export function verify(root = repositoryRoot) {
  for (const relative of REQUIRED_ARTIFACTS) assert.ok(existsSync(join(root, relative)), `missing ${relative}`);

  const css = read(root, 'ui/packages/design-tokens/dist/tokens.css');

  // ---- assertions that predate the tree walk (V5), unchanged ----
  for (const token of ['--color-background', '--color-foreground', '--color-primary', '--color-danger', '--space-4', '--focus-ring']) {
    assert.match(css, new RegExp(`${token.replaceAll('-', '\\-')}\\s*:`), `missing token ${token}`);
  }
  assert.match(css, /\.theme-dark|\[data-theme="dark"\]/, 'missing dark theme');
  assert.match(css, /prefers-reduced-motion/, 'missing reduced-motion fallback');

  const components = read(root, 'ui/packages/react-ui/src/components.tsx');
  const styles = read(root, 'ui/packages/react-ui/styles.css');
  for (const marker of ['aria-busy', 'aria-invalid', 'aria-live']) assert.match(components, new RegExp(marker), `missing accessibility marker ${marker}`);
  assert.match(styles, /:focus-visible/, 'missing focus-visible styling');
  assert.match(components, /disabled|aria-disabled/, 'missing disabled behavior');
  assert.match(components, /role=["']dialog["']|role: ["']dialog["']/, 'missing dialog role');

  // ---- new: the token source is singular ----
  const source = JSON.parse(read(root, 'ui/tokens/tokens.json'));
  const tokens = resolveAliases(readTokens(source)); // throws ui-alias-unresolved
  const declaredNames = new Set(tokens.map((token) => token.name));
  const byName = new Map(tokens.map((token) => [token.name, token]));
  const emitted = parseCustomProperties(css);
  const emittedNames = new Set(emitted.map((property) => property.name));

  for (const token of tokens) {
    if (!emittedNames.has(token.name)) {
      fail('ui-token-dropped', `\`${token.path}\` (${token.name}) is declared in tokens.json but not emitted in tokens.css`);
    }
  }
  for (const property of emitted) {
    if (!declaredNames.has(property.name)) {
      fail('ui-undeclared-token', `tokens.css:${property.line} declares ${property.name}, which no $value in tokens.json declares`);
    }
  }

  // TypeScript artifact carries every declared token too, by name and value.
  const ts = read(root, 'ui/packages/design-tokens/dist/tokens.ts');
  const valueMap = /export const tokenValues = \{([\s\S]*?)\} as const;/.exec(ts);
  if (!valueMap) fail('ui-token-dropped', 'tokens.ts does not export the tokenValues name -> value map');
  const tsValues = new Map(
    [...valueMap[1].matchAll(/"(--[A-Za-z0-9-]+)":\s*"((?:[^"\\]|\\.)*)"/g)].map((match) => [match[1], match[2]]),
  );
  for (const token of tokens) {
    if (!tsValues.has(token.name)) {
      fail('ui-token-dropped', `\`${token.path}\` (${token.name}) is declared in tokens.json but not emitted in tokens.ts`);
    }
  }
  for (const [name, value] of tsValues) {
    if (!declaredNames.has(name)) fail('ui-undeclared-token', `tokens.ts declares ${name}, which no $value in tokens.json declares`);
    if (byName.get(name).literal !== value) {
      fail('ui-orphan-literal', `tokens.ts declares ${name} as "${value}" but tokens.json resolves it to "${byName.get(name).literal}"`);
    }
  }

  // Framework themes consume a complete semantic color map for each supported
  // mode. Values must resolve to the source token tree, not to a second palette.
  const themeMap = /export const semanticColorValues = \{([\s\S]*?)\} as const;/.exec(ts);
  if (!themeMap) fail('ui-theme-map-missing', 'tokens.ts does not export semanticColorValues for framework theme adapters');
  const semanticColors = new Map(
    tokens.filter((token) => token.path.startsWith('semantic.color.'))
      .map((token) => [token.path.slice('semantic.color.'.length), token]),
  );
  const expectedByMode = { light: new Map([...semanticColors].map(([name, token]) => [name, token.literal])), dark: new Map([...semanticColors].map(([name, token]) => [name, token.literal])) };
  for (const { mode, groups } of readThemeOverrides(source)) {
    for (const group of groups.filter((item) => item.group === 'color')) {
      for (const { key, alias } of group.entries) {
        const target = tokens.find((token) => token.path === alias);
        if (!target) fail('ui-theme-map-invalid', `${mode}.${key} references undeclared token {${alias}}`);
        expectedByMode[mode]?.set(key, target.literal);
      }
    }
  }
  for (const mode of ['light', 'dark']) {
    const modeObject = new RegExp(`(?:^|[, ])${mode}: \\{([^{}]*)\\}`).exec(themeMap[1]);
    if (!modeObject) fail('ui-theme-map-missing', `tokens.ts does not emit the ${mode} semantic theme`);
    const actual = new Map([...modeObject[1].matchAll(/([A-Za-z_$][A-Za-z0-9_$]*):\s*"((?:[^"\\]|\\.)*)"/g)].map((match) => [match[1], match[2]]));
    for (const [name, value] of expectedByMode[mode]) {
      if (actual.get(name) !== value) fail('ui-theme-map-invalid', `${mode}.${name} is ${actual.get(name) ?? 'missing'}, expected ${value} from tokens.json`);
    }
    for (const name of actual.keys()) {
      if (!expectedByMode[mode].has(name)) fail('ui-theme-map-invalid', `${mode}.${name} is not a semantic color in tokens.json`);
    }
  }

  const templatePackage = JSON.parse(read(root, 'ui/packages/react-templates/package.json'));
  if (templatePackage.name !== '@platform/react-templates') fail('ui-template-package-invalid', 'react-templates package name does not match @platform/react-templates');
  if (templatePackage.exports?.['.'] !== './src/index.ts' || templatePackage.exports?.['./styles.css'] !== './styles.css') {
    fail('ui-template-package-invalid', '@platform/react-templates must expose its public entry point and stylesheet');
  }
  if (!templatePackage.peerDependencies?.antd) fail('ui-template-package-invalid', '@platform/react-templates must declare Ant Design as a peer dependency');
  if (!templatePackage.peerDependencies?.['@types/react']) fail('ui-template-package-invalid', '@platform/react-templates must declare React typings as a peer dependency');
  if (!templatePackage.dependencies?.['@platform/react-ui'] || !templatePackage.dependencies?.['@platform/react-shell']) {
    fail('ui-template-package-invalid', '@platform/react-templates must compose the existing platform UI and shell packages');
  }

  const layouts = read(root, 'ui/packages/react-templates/src/layouts.tsx');
  const templateTheme = read(root, 'ui/packages/react-templates/src/theme.tsx');
  const templateStyles = read(root, 'ui/packages/react-templates/styles.css');
  for (const component of ['PublicPageLayout', 'UserDashboardLayout', 'AdminDashboardLayout', 'DataTable']) {
    if (!layouts.includes(`function ${component}`)) fail('ui-template-export-missing', `${component} is not implemented in react-templates`);
  }
  if (/export\s+(?:function|const)\s+(?:Button|Field|Card|Badge|State)\b/.test(layouts)) {
    fail('ui-template-duplicate-primitive', 'react-templates exports a primitive already owned by @platform/react-ui');
  }
  for (const marker of ['platform-skip-link', 'aria-label="Main navigation"', 'AppShell', 'role="region"', 'tabIndex={0}']) {
    if (!layouts.includes(marker)) fail('ui-template-accessibility-missing', `react-templates is missing ${marker}`);
  }
  for (const marker of ["kind=\"forbidden\"", 'headingRef.current?.focus()', 'onRetry &&']) {
    if (!layouts.includes(marker)) fail('ui-template-state-missing', `react-templates is missing ${marker}`);
  }
  for (const marker of ['semanticColorValues', 'tokenValues', 'colorPrimary', 'controlOutline']) {
    if (!templateTheme.includes(marker)) fail('ui-template-theme-missing', `react-templates theme does not map ${marker}`);
  }
  if (/#(?:[0-9a-f]{3,8})\b|\brgb\(/i.test(templateTheme) || /#[0-9a-f]{3,8}\b|\brgb\(/i.test(templateStyles)) {
    fail('ui-template-color-literal', 'react-templates contains a color literal instead of a semantic token');
  }
  if (!templateStyles.includes('@media (max-width: 48rem)') || !templateStyles.includes('@media (max-width: 30rem)')) {
    fail('ui-template-breakpoint-missing', 'react-templates must use the declared responsive breakpoint scale');
  }
  for (const marker of ['.platform-data-table-region > .ant-table-wrapper', 'minmax(0, 1fr)']) {
    if (!templateStyles.includes(marker)) fail('ui-template-responsive-contract', `react-templates styles are missing ${marker}`);
  }
  if (templateStyles.includes('@import')) {
    fail('ui-template-stylesheet-import', 'react-templates stylesheet must not rely on paths outside its packed artifact');
  }

  // V6: no emitted token from 0.1.0 is dropped or renamed (design.md section 5, D15).
  const baseline = JSON.parse(read(root, 'ui/tokens/emitted-baseline.json'));
  for (const [name, value] of Object.entries(baseline.tokens)) {
    const property = emitted.find((candidate) => candidate.name === name);
    if (!property) {
      fail('ui-token-renamed', `${name} was emitted by the 0.1.0 generator and is no longer emitted; renaming an emitted token is a contract change, not a refactor`);
    }
    if (property.value !== value) {
      fail('ui-token-renamed', `${name} was ${value} at 0.1.0 and is now ${property.value}`);
    }
    if (!value.startsWith('var(') && tsValues.get(name) !== value) {
      fail('ui-token-renamed', `${name} resolved to ${value} at 0.1.0 and now resolves to ${tsValues.get(name)} in tokens.ts`);
    }
  }

  // No literal in the generated CSS that is not reachable from a $value or a documented
  // var() alias. Scoped to custom-property declarations, which is where token values live:
  // the canonical primitive's own 1px/-1px geometry and the reduced-motion durations are
  // fixed CSS, not token values.
  const literals = declaredLiterals(tokens);
  for (const property of emitted) {
    const reference = /^var\((--[A-Za-z0-9-]+)\)$/.exec(property.value);
    if (reference) {
      if (!declaredNames.has(reference[1])) {
        fail('ui-orphan-literal', `tokens.css:${property.line} ${property.name} references ${reference[1]}, which no $value declares`);
      }
      continue;
    }
    if (!literals.has(property.value)) {
      fail('ui-orphan-literal', `tokens.css:${property.line} ${property.name}: ${property.value} is not reachable from any $value in tokens.json`);
    }
  }

  // Every theme override resolves to a declared primitive (the dark theme used to carry
  // five literals that existed in no token file).
  for (const { mode, groups } of readThemeOverrides(source)) {
    for (const group of groups) {
      for (const { key, alias } of group.entries) {
        if (!alias || !tokens.some((token) => token.path === alias)) {
          fail('ui-alias-unresolved', `theme.${mode}.${group.group}.${key} references ${alias ?? key}, which is not a declared token`);
        }
      }
    }
  }

  // ---- new: the canonical visually-hidden primitive ----
  const definitions = PRIMITIVE_BEARING_ARTIFACTS.filter((relative) => /\.visually-hidden[^{}]*\{/.test(read(root, relative)));
  if (definitions.length > 1) {
    fail('ui-duplicate-primitive', `.visually-hidden is defined in ${definitions.join(' and ')}; the generated artifacts must define it exactly once`);
  }
  if (definitions.length === 0) {
    fail('ui-primitive-incomplete', `no ${CANONICAL_VISUALLY_HIDDEN.selector} definition found in ${PRIMITIVE_BEARING_ARTIFACTS.join(', ')}`);
  }
  for (const relative of PRIMITIVE_BEARING_MODULES) {
    if (new RegExp(`\\.visually-hidden[^{}]*\\{`).test(read(root, relative))) {
      fail('ui-duplicate-primitive', `${relative} defines .visually-hidden; only ${CANONICAL_VISUALLY_HIDDEN.selector} in tokens.css may define it`);
    }
  }

  const body = ruleBody(css, CANONICAL_VISUALLY_HIDDEN.selector);
  if (body === undefined) fail('ui-primitive-incomplete', 'tokens.css does not define .visually-hidden');
  const found = declarations(body);
  const important = found.filter(([, value]) => value.includes('!important'));
  if (important.length > 0) {
    fail('ui-primitive-incomplete', `.visually-hidden uses !important on ${important.map(([property]) => property).join(', ')}; the canonical rule stays overridable`);
  }
  const missing = CANONICAL_VISUALLY_HIDDEN.declarations.filter(([property, value]) => !found.some(([p, v]) => p === property && v === value));
  if (missing.length > 0) {
    fail('ui-primitive-incomplete', `.visually-hidden is missing ${missing.map(([property, value]) => `${property}: ${value}`).join('; ')}`);
  }

  return {
    artifacts: REQUIRED_ARTIFACTS.length,
    declaredTokens: tokens.length,
    emittedTokens: emittedNames.size,
    baselineTokens: Object.keys(baseline.tokens).length,
    primitive: `${CANONICAL_VISUALLY_HIDDEN.selector} (${CANONICAL_VISUALLY_HIDDEN.declarations.length} properties, no !important)`,
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const report = verify();
  console.log(
    `UI verification passed: ${report.artifacts} required artifacts and accessibility markers present; ` +
      `${report.declaredTokens} declared tokens emitted (${report.emittedTokens} in tokens.css, ${report.baselineTokens} baselined at 0.1.0); ` +
      `canonical primitive ${report.primitive}.`,
  );
}
