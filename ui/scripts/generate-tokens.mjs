// Generate ui/packages/design-tokens/dist/{tokens.css,tokens.ts} and the Tailwind
// projections dist/tailwind/{preset.js,theme.css} from ui/tokens/tokens.json.
//
// The generator walks the DTCG tree and resolves its aliases. It enumerates no
// token path and hardcodes no token value: the previous template literal read
// specific paths (source.color.slate['50'].$value, …), so a token added to
// tokens.json produced no output and no error, and the dark theme carried a
// second literal palette that existed in no token file (design.md D-A, D-B).
// verify-ui.mjs fails both of those states; this file can no longer cause them.

import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  CANONICAL_VISUALLY_HIDDEN,
  formatDeclarations,
  readThemeOverrides,
  readTokens,
  resolveAliases,
  semanticColorTokens,
} from './lib/token-model.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const source = JSON.parse(await readFile(join(root, 'tokens/tokens.json'), 'utf8'));

const tokens = resolveAliases(readTokens(source));
const semantics = semanticColorTokens(tokens);

function customPropertyOf(semanticName) {
  const token = semantics.get(semanticName);
  if (!token) throw new Error(`generate-tokens: no semantic color token named ${semanticName}`);
  return token;
}

/** One blank-line-separated group per top-level token group, in source order. */
function groupedByGroup(members) {
  const groups = [];
  for (const token of members) {
    let current = groups.at(-1);
    if (!current || current.group !== token.group) {
      current = { group: token.group, members: [] };
      groups.push(current);
    }
    current.members.push(token);
  }
  return groups.map((group) => `\n${formatDeclarations(group.members)}`);
}

// Theme blocks are data: the override map in tokens.json decides which declared
// primitive each semantic name re-points to in each mode.
const themeBlocks = readThemeOverrides(source).map(({ mode, groups }) => {
  const declarations = groups.flatMap((group) =>
    group.entries.map(({ key, alias }) => {
      const token = customPropertyOf(key);
      const referenced = tokens.find((candidate) => candidate.path === alias);
      if (!referenced) {
        throw new Error(`generate-tokens: theme override ${mode} for ${token.path} references undeclared {${alias}}`);
      }
      return `  ${token.name}: var(${referenced.name});`;
    }),
  );
  return `[data-theme="${mode}"], .theme-${mode} {\n${declarations.join('\n')}\n}`;
});

const rootCss = [
  ...groupedByGroup(tokens.filter((token) => !token.path.startsWith('semantic.'))),
  ...groupedByGroup(tokens.filter((token) => token.path.startsWith('semantic.'))),
].join('\n');
const rootBlock = `:root {\n${rootCss}\n}`;

const visuallyHidden = `${CANONICAL_VISUALLY_HIDDEN.selector} {\n${CANONICAL_VISUALLY_HIDDEN.declarations
  .map(([property, value]) => `  ${property}: ${value};`)
  .join('\n')}\n}`;

const css = [
  '/* Generated from ui/tokens/tokens.json. Do not edit. */',
  rootBlock,
  ...themeBlocks.map((block) => `\n${block}`),
  `\n/* Canonical shared primitive. Emitted here so it cannot drift from the kit;`,
  '   no !important, so a consumer cascade can still override it. */',
  `${visuallyHidden}`,
  '@media (prefers-reduced-motion: reduce) { *, *::before, *::after { animation-duration: 0.01ms !important; animation-iteration-count: 1 !important; scroll-behavior: auto !important; transition-duration: 0.01ms !important; } }',
  '',
].join('\n');

// tokens.ts keeps the pre-rewrite `tokens` shape and names, and adds a flat
// resolved map so the TypeScript artifact carries every declared value too.
const groups = [
  ['color', tokens.filter((token) => token.path.startsWith('semantic.color.'))],
  ['space', tokens.filter((token) => token.group === 'space')],
  ['radius', tokens.filter((token) => token.group === 'radius')],
  ['font', tokens.filter((token) => token.group === 'font')],
  ['component', tokens.filter((token) => token.group === 'component')],
];

function objectLiteral(entries) {
  return `{ ${entries.map(([key, value]) => `${keyLiteral(key)}: ${value}`).join(', ')} }`;
}

const keyLiteral = (key) => (/^(?:[A-Za-z_$][A-Za-z0-9_$]*|\d+)$/.test(key) ? key : JSON.stringify(key));

// `semantic.color.mutedForeground` is a color token, not a `semantic.color.*` member of a
// `color.*` primitive group, so the color group keys off the semantic prefix.
const keyOf = (group) => (token) => {
  const prefix = group === 'color' ? 'semantic.color.' : `${group}.`;
  return token.path.slice(prefix.length);
};

const tokensObject = objectLiteral(
  groups.map(([group, members]) => [group, objectLiteral(members.map((token) => [keyOf(group)(token), JSON.stringify(`var(${token.name})`)]))]),
);

const tokenValues = objectLiteral(tokens.map((token) => [token.name, JSON.stringify(token.literal)]));

// Concrete semantic values for framework theme adapters. Theme overrides are
// resolved from the same source tree as CSS, so adapters never carry a second
// light/dark palette.
const semanticValuesByMode = { light: new Map(), dark: new Map() };
for (const [name, token] of semantics) semanticValuesByMode.light.set(name, token.literal);
for (const [name, value] of semanticValuesByMode.light) semanticValuesByMode.dark.set(name, value);
for (const { mode, groups } of readThemeOverrides(source)) {
  if (!semanticValuesByMode[mode]) semanticValuesByMode[mode] = new Map(semanticValuesByMode.light);
  for (const group of groups) {
    if (group.group !== 'color') continue;
    for (const { key, alias } of group.entries) {
      const target = tokens.find((token) => token.path === alias);
      if (!target) throw new Error(`generate-tokens: theme override ${mode} for ${key} references undeclared {${alias}}`);
      semanticValuesByMode[mode].set(key, target.literal);
    }
  }
}
const semanticValuesObject = objectLiteral(
  Object.entries(semanticValuesByMode).map(([mode, values]) => [
    mode,
    objectLiteral([...values].map(([name, value]) => [name, JSON.stringify(value)])),
  ]),
);

const ts = [
  '/** Generated from tokens.json. Do not edit. */',
  `export const tokens = ${tokensObject} as const;`,
  '',
  '/** Every declared token name and its alias-resolved value. */',
  `export const tokenValues = ${tokenValues} as const;`,
  '',
  '/** Semantic color values resolved from tokens.json for framework theme adapters. */',
  `export const semanticColorValues = ${semanticValuesObject} as const;`,
  '',
  'export type TokenName = keyof typeof tokenValues;',
  'export type PlatformTheme = \'light\' | \'dark\';',
  'export type SemanticColorName = keyof typeof semanticColorValues.light;',
  '',
].join('\n');

const output = join(root, 'packages/design-tokens/dist');
await mkdir(output, { recursive: true });
await writeFile(join(output, 'tokens.css'), css);
await writeFile(join(output, 'tokens.ts'), ts);

// ---- Tailwind projections (openspec change ui-design-guideline-adoption, D-5) ----
// Pure projections of tokens.json: every entry is a var() reference to an emitted
// custom property, neither file may contain a color literal, and no token path is
// enumerated here — the color surface is "every semantic color, plus any color-group
// token that aliases a semantic one" (the --color-focus-ring bridge), in declaration
// order so a later bridge entry overrides the key it shares.

/** `--color-muted-foreground` -> `muted-foreground`, `--focus-ring` -> `focus-ring`. */
function utilityKeyOf(token, namespace = '--color-') {
  return token.name.startsWith(namespace) ? token.name.slice(namespace.length) : token.name.slice(2);
}

/** Dedupe by utility key, later entries win: the `--color-focus-ring` bridge
 *  overrides the key the legacy `--focus-ring` semantic would produce, so both
 *  Tailwind artifacts cite the documented name and carry no duplicate member. */
function project(tokensForKeys, namespace, render) {
  const byKey = new Map();
  for (const token of tokensForKeys) byKey.set(utilityKeyOf(token, namespace), render(token));
  return byKey;
}

const colorProjections = [
  ...semantics.values(),
  ...tokens.filter((token) => token.group === 'color' && token.alias?.startsWith('semantic.')),
];

const fontFamilyTokens = tokens.filter((token) => token.type === 'fontFamily');
const spaceTokens = tokens.filter((token) => token.group === 'space');
const radiusTokens = tokens.filter((token) => token.group === 'radius');

const colorEntries = project(colorProjections, '--color-', (token) => JSON.stringify(`var(${token.name})`));
const fontEntries = project(fontFamilyTokens, '--font-', (token) => `[${JSON.stringify(`var(${token.name})`)}]`);
const spacingEntries = project(spaceTokens, '--space-', (token) => JSON.stringify(`var(${token.name})`));
const radiusEntries = project(radiusTokens, '--radius-', (token) => JSON.stringify(`var(${token.name})`));

// The kit's custom properties hold hex, not color channels, so the
// `rgb(var(--x) / <alpha-value>)` pattern is unavailable without re-cutting every
// color token; the v3 preset therefore uses plain var() references and kit colors
// do not take opacity modifiers. Documented in docs/ui-tokens.md.
const preset = [
  '/** Generated from ui/tokens/tokens.json. Do not edit. */',
  '/** Tailwind v3 preset. Colors are var() references of the kit custom properties,',
  ' *  so utilities follow the dark theme at runtime. This package is ESM',
  ' *  ("type": "module"): import this preset from a tailwind.config.mjs. */',
  `const preset = { colors: { ${[...colorEntries].map(([key, value]) => `${keyLiteral(key)}: ${value}`).join(', ')} }, fontFamily: { ${[...fontEntries].map(([key, value]) => `${keyLiteral(key)}: ${value}`).join(', ')} }, spacing: { ${[...spacingEntries].map(([key, value]) => `${keyLiteral(key)}: ${value}`).join(', ')} }, borderRadius: { ${[...radiusEntries].map(([key, value]) => `${keyLiteral(key)}: ${value}`).join(', ')} } };`,
  '',
  'export default preset;',
  '',
].join('\n');

// `@theme inline` is load-bearing: without `inline`, Tailwind re-emits the @theme
// variables into :root and competes with tokens.css (and its dark-theme override)
// for the same names. With `inline`, utilities substitute the var() reference and
// tokens.css stays the single source per mode. Import order: tokens.css, then this.
const theme = [
  '/* Generated from ui/tokens/tokens.json. Do not edit. */',
  '/* Tailwind v4 theme bridge. Import after dist/tokens.css so every',
  '   utility resolves through the kit custom properties and follows the',
  '   dark theme and the reduced-motion fallback defined there. */',
  '@theme inline {',
  ...[...colorEntries].map(([key]) => `  --color-${key}: var(--color-${key});`),
  ...[...fontEntries].map(([key]) => `  --font-${key}: var(--font-${key});`),
  ...[...spacingEntries].map(([key]) => `  --spacing-${key}: var(--space-${key});`),
  ...[...radiusEntries].map(([key]) => `  --radius-${key}: var(--radius-${key});`),
  '}',
  '',
].join('\n');

const tailwindOutput = join(output, 'tailwind');
await mkdir(tailwindOutput, { recursive: true });
await writeFile(join(tailwindOutput, 'preset.js'), preset);
await writeFile(join(tailwindOutput, 'theme.css'), theme);

console.log(
  `generate-tokens: emitted ${tokens.length} tokens, ${themeBlocks.length} theme block(s), ` +
    `${colorProjections.length} color/${fontFamilyTokens.length} font/${spaceTokens.length} spacing/${radiusTokens.length} radius Tailwind mappings from ui/tokens/tokens.json.`,
);
