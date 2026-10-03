// Generate ui/packages/design-tokens/dist/{tokens.css,tokens.ts} from
// ui/tokens/tokens.json.
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

const ts = [
  '/** Generated from tokens.json. Do not edit. */',
  `export const tokens = ${tokensObject} as const;`,
  '',
  '/** Every declared token name and its alias-resolved value. */',
  `export const tokenValues = ${tokenValues} as const;`,
  '',
  'export type TokenName = keyof typeof tokenValues;',
  'export type PlatformTheme = \'light\' | \'dark\';',
  '',
].join('\n');

const output = join(root, 'packages/design-tokens/dist');
await mkdir(output, { recursive: true });
await writeFile(join(output, 'tokens.css'), css);
await writeFile(join(output, 'tokens.ts'), ts);

console.log(`generate-tokens: emitted ${tokens.length} tokens, ${themeBlocks.length} theme block(s) from ui/tokens/tokens.json.`);
