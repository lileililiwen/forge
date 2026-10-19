// DTCG token model for the platform UI layer.
//
// One walk, one naming rule, one alias resolver — shared by the generator
// (ui/scripts/generate-tokens.mjs) and the verifier (ui/scripts/verify-ui.mjs)
// so the two cannot disagree about what a token is or what it is called.
//
// The model holds no design-token value and no token name of its own. Every
// value it returns is read from ui/tokens/tokens.json, and the only hardcoded
// name in this file is the documented V6 rename exception below.

/** Structural keys in the DTCG source that are not tokens. */
const META_KEYS = new Set(['$value', '$type', '$description', '$extensions', '$deprecated']);

/** Non-token top-level block: theme override map, keyed by semantic token. */
const OVERRIDE_BLOCK = 'theme';

/**
 * Names emitted by the pre-rewrite 0.1.0 generator that do not follow from
 * their source path. Kept verbatim because the rewrite must not rename an
 * emitted token (design.md section 5); dropping this entry would remove
 * `--focus-ring`, which react-ui/styles.css and Platform.UI.Razor consume.
 */
const LEGACY_NAME_OVERRIDES = {
  'semantic.color.focusRing': 'focus-ring',
};

/** The canonical visually-hidden primitive, emitted by the generator (spec requirement 3). */
const CANONICAL_VISUALLY_HIDDEN = {
  selector: '.visually-hidden',
  declarations: [
    ['position', 'absolute'],
    ['width', '1px'],
    ['height', '1px'],
    ['padding', '0'],
    ['margin', '-1px'],
    ['overflow', 'hidden'],
    ['clip', 'rect(0 0 0 0)'],
    ['white-space', 'nowrap'],
    ['border', '0'],
  ],
};

export class TokenCheckError extends Error {
  constructor(code, message) {
    super(`${code}: ${message}`);
    this.name = 'TokenCheckError';
    this.code = code;
  }
}

function fail(code, message) {
  throw new TokenCheckError(code, message);
}

function kebab(segment) {
  return String(segment)
    .replace(/([a-z0-9])([A-Z])/g, '$1-$2')
    .replace(/[^A-Za-z0-9]+/g, '-')
    .toLowerCase();
}

/**
 * Custom-property name for a source path.
 *
 * `color.slate.50` -> `--color-slate-50`, `space.4` -> `--space-4`,
 * `semantic.color.mutedForeground` -> `--color-muted-foreground`,
 * `component.button.minHeight` -> `--component-button-min-height`.
 */
export function customPropertyName(path) {
  const key = path.join('.');
  if (Object.hasOwn(LEGACY_NAME_OVERRIDES, key)) return `--${LEGACY_NAME_OVERRIDES[key]}`;
  const segments = path[0] === 'semantic' ? path.slice(1) : path;
  return `--${segments.map(kebab).join('-')}`;
}

/** `{alias}` -> referenced source path; anything else is a literal value. */
function aliasTarget(value) {
  const match = /^\{([^}]+)\}$/.exec(value);
  return match ? match[1] : null;
}

function isToken(node) {
  return node !== null && typeof node === 'object' && !Array.isArray(node) && typeof node.$value === 'string';
}

/**
 * Walk the DTCG tree and return every declared token, in source order.
 * Each entry: { path, name, value, alias, type, cssValue, literal } where
 * `cssValue` is the emitted right-hand side (a literal or `var(--name)`) and
 * `literal` is the value with every alias chain resolved.
 */
export function readTokens(source) {
  const tokens = [];

  const visit = (node, path) => {
    for (const [key, child] of Object.entries(node)) {
      if (META_KEYS.has(key) || key.startsWith('$')) continue;
      const childPath = [...path, key];
      if (isToken(child)) {
        const alias = aliasTarget(child.$value);
        tokens.push({
          path: childPath.join('.'),
          name: customPropertyName(childPath),
          value: child.$value,
          alias,
          type: typeof child.$type === 'string' ? child.$type : undefined,
          group: path[0] ?? key,
        });
      } else if (child !== null && typeof child === 'object' && !Array.isArray(child)) {
        visit(child, childPath);
      }
    }
  };

  for (const [key, child] of Object.entries(source)) {
    if (key.startsWith('$') || key === OVERRIDE_BLOCK) continue;
    if (child !== null && typeof child === 'object' && !Array.isArray(child)) visit(child, [key]);
  }

  return tokens;
}

/** Theme override maps: `{ 'dark': { color: { background: '{color.slate.900}', ... } } }`. */
export function readThemeOverrides(source) {
  const block = source[OVERRIDE_BLOCK];
  if (block === undefined) return [];
  return Object.entries(block)
    .filter(([key]) => !key.startsWith('$'))
    .map(([mode, groups]) => ({
      mode,
      groups: Object.entries(groups).map(([group, entries]) => ({
        group,
        entries: Object.entries(entries).map(([key, value]) => ({ key, value, alias: aliasTarget(String(value)) })),
      })),
    }));
}

/**
 * Resolve alias chains for every token. Fails `ui-alias-unresolved` naming the
 * alias and the token that references it (spec requirement 2).
 */
export function resolveAliases(tokens) {
  const byPath = new Map(tokens.map((token) => [token.path, token]));

  const literalOf = (token, seen) => {
    if (!token.alias) return token.value;
    if (seen.has(token.path)) {
      fail('ui-alias-unresolved', `alias cycle: ${[...seen, token.path].join(' -> ')} (referenced by ${[...seen][0]})`);
    }
    const target = byPath.get(token.alias);
    if (!target) {
      fail('ui-alias-unresolved', `\`${token.path}\` references alias {${token.alias}}, which is not a declared token`);
    }
    seen.add(token.path);
    const literal = literalOf(target, seen);
    seen.delete(token.path);
    return literal;
  };

  return tokens.map((token) => {
    if (token.alias && !byPath.has(token.alias)) {
      fail('ui-alias-unresolved', `\`${token.path}\` references alias {${token.alias}}, which is not a declared token`);
    }
    const literal = literalOf(token, new Set());
    return {
      ...token,
      literal,
      cssValue: token.alias ? `var(${customPropertyName(token.alias.split('.'))})` : token.value,
    };
  });
}

/** Every literal value a generated artifact is allowed to contain. */
export function declaredLiterals(tokens) {
  return new Set(tokens.map((token) => token.literal));
}

/** Semantic tokens, keyed by semantic name (used by the dark-theme override map). */
export function semanticColorTokens(tokens) {
  return new Map(
    tokens
      .filter((token) => token.path.startsWith('semantic.color.'))
      .map((token) => [token.path.slice('semantic.color.'.length), token]),
  );
}

/** Parse `--name: value;` custom-property declarations out of CSS text, with line numbers. */
export function parseCustomProperties(css) {
  const found = [];
  const lines = css.split('\n');
  lines.forEach((line, index) => {
    for (const match of line.matchAll(/(--[A-Za-z0-9-]+)\s*:\s*([^;]+);/g)) {
      found.push({ name: match[1], value: match[2].trim(), line: index + 1, text: line.trim() });
    }
  });
  return found;
}

/** Format a token list as a CSS declaration block body, one declaration per line. */
export function formatDeclarations(tokens) {
  return tokens.map((token) => `  ${token.name}: ${token.cssValue};`).join('\n');
}

export { CANONICAL_VISUALLY_HIDDEN, LEGACY_NAME_OVERRIDES };
