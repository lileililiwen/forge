import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { parseBranding, orderNavigation, groupFooterLinks, BrandingError } from '../src/branding.ts';
import { readThemeOverrides, readTokens, resolveAliases } from '../../../scripts/lib/token-model.mjs';

const forms = await readFile(new URL('../src/auth-forms.tsx', import.meta.url), 'utf8');
const theme = await readFile(new URL('../src/theme.tsx', import.meta.url), 'utf8');
const branding = await readFile(new URL('../src/branding.ts', import.meta.url), 'utf8');
const styles = await readFile(new URL('../styles.css', import.meta.url), 'utf8');
const primitives = await readFile(new URL('../../react-ui/src/components.tsx', import.meta.url), 'utf8');
const tokenSource = JSON.parse(await readFile(new URL('../../../tokens/tokens.json', import.meta.url), 'utf8'));
const schema = JSON.parse(await readFile(new URL('../../../schemas/branding.schema.json', import.meta.url), 'utf8'));
const index = await readFile(new URL('../src/index.ts', import.meta.url), 'utf8');

/** Strip comments so a negative assertion judges the code, not the prose describing it. */
function code(source) {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/(^|[^:'"\\])\/\/.*$/gm, '$1')
    .replace(/^\s*\/\/.*$/gm, '');
}
const formsCode = code(forms);
const themeCode = code(theme);

// --- Form state transitions -------------------------------------------------

for (const form of ['LoginForm', 'RegisterForm', 'ForgotPasswordForm']) {
  assert.match(forms, new RegExp(`export function ${form}\\b`), `${form} is not exported`);
}
for (const state of ['idle', 'invalid', 'submitting', 'success', 'error']) {
  assert.match(forms, new RegExp(`'${state}'`), `form state ${state} is not declared`);
}
assert.match(forms, /setState\('invalid'\)/, 'a failed local check must move the form to invalid');
assert.match(forms, /setState\('submitting'\)/, 'submission must move the form to submitting');
assert.match(forms, /setState\('success'\)/, 'an ok result must move the form to success');
assert.match(forms, /setState\('error'\)/, 'a failed result must move the form to error');
assert.match(forms, /Object\.keys\(invalid\)\.length > 0/, 'invalid must be reached without calling the handler');
assert.match(forms, /catch \(thrown\)/, 'a throwing handler must still reach error');
assert.match(forms, /loading=\{state === 'submitting'\}/, 'the submit control must show the pending state');
assert.match(forms, /export function LogoutControl/, 'the logout control is not exported');
assert.match(forms, /onLogout: \(\) => void/, 'logout must take an explicit sign-out handler');

// --- Composition over existing primitives ------------------------------------

assert.match(forms, /from '@platform\/react-ui'/, 'forms must consume the shared primitives');
for (const primitive of ['Button', 'Card', 'Field', 'State']) {
  assert.match(forms, new RegExp(`\\b${primitive}\\b`), `${primitive} is not composed`);
}
assert.doesNotMatch(formsCode, /<input/, 'react-auth must not hand-roll an input');
assert.doesNotMatch(formsCode, /<label/, 'react-auth must not hand-roll a label');
assert.doesNotMatch(formsCode, /aria-describedby/, 'Field already owns aria-describedby');
assert.doesNotMatch(formsCode, /aria-invalid/, 'Field already owns aria-invalid');
assert.match(index, /export \* from '.\/auth-forms'/, 'the package root must export the forms');
assert.match(index, /export \* from '.\/theme'/, 'the package root must export the theme surface');
assert.match(index, /export \* from '.\/branding'/, 'the package root must export branding');

// --- Theme: persistence, system default, keyboard, announcement --------------

assert.match(theme, /export function ThemeProvider/, 'ThemeProvider is not exported');
assert.match(theme, /export function useTheme/, 'useTheme is not exported');
assert.match(theme, /document\.documentElement\.dataset\.theme = theme/, 'the theme must be set on <html>');
assert.match(theme, /\(prefers-color-scheme: dark\)/, 'the system preference must be the default');
assert.match(theme, /const theme: ThemeName = explicitTheme \?\? system \?\? fallback/, 'a stored choice must win over the system preference');
assert.match(theme, /storage\?\.setItem\(THEME_STORAGE_KEY, next\)/, 'an explicit choice must be persisted');
assert.match(theme, /storage\?\.removeItem\(THEME_STORAGE_KEY\)/, 'the stored choice must be clearable');
assert.match(theme, /export const THEME_STORAGE_KEY = 'platform\.theme'/, 'the storage key must be stable');
assert.match(theme, /aria-pressed=\{theme === 'dark'\}/, 'the current theme must be exposed as state');
assert.match(theme, /aria-label=\{`\$\{label\}: \$\{theme\}`\}/, 'the theme name must be announced as text');
assert.match(theme, /onClick=\{\(\) => setTheme\(theme === 'dark' \? 'light' : 'dark'\)\}/, 'the toggle must be keyboard operable');
assert.match(theme, /SkipToContent/, 'the skip link is not rendered');
assert.match(theme, /href="#main-content"/, 'the skip link must target the main content');

// --- Branding override of logo, navigation, and footer -----------------------

assert.equal(schema.title, 'branding.json');
assert.deepEqual(schema.required.sort(), ['footerLinks', 'locale', 'logo', 'name', 'navigation']);
for (const field of ['name', 'logo', 'theme', 'navigation', 'footerLinks', 'locale']) {
  assert.ok(schema.properties[field], `branding.schema.json is missing ${field}`);
}
assert.deepEqual(schema.properties.theme.enum, ['light', 'dark']);
for (const field of ['id', 'label', 'href', 'order']) {
  assert.ok(schema.properties.navigation.items.properties[field], `navigation[] is missing ${field}`);
}
assert.ok(schema.properties.navigation.items.properties.icon, 'navigation[] must allow an optional icon');
for (const field of ['id', 'label', 'href', 'group']) {
  assert.ok(schema.properties.footerLinks.items.properties[field], `footerLinks[] is missing ${field}`);
}

const valid = {
  name: 'Example',
  logo: { kind: 'path', src: '/logo.svg' },
  theme: 'dark',
  navigation: [
    { id: 'later', label: 'Later', href: '/later', order: 2 },
    { id: 'first', label: 'First', href: '/first', order: 1 },
  ],
  footerLinks: [
    { id: 'a', label: 'Alpha', href: 'https://example.com/a', group: 'Partners' },
    { id: 'b', label: 'Beta', href: 'https://example.com/b', group: 'Partners' },
    { id: 'c', label: 'Gamma', href: 'https://example.com/c', group: 'Legal' },
  ],
  locale: 'en-GB',
};
const parsed = parseBranding(valid);
assert.equal(parsed.name, 'Example', 'name must come from the document');
assert.equal(parsed.theme, 'dark', 'theme must come from the document');
assert.equal(parsed.locale, 'en-GB');
assert.deepEqual(parsed.logo, { kind: 'path', src: '/logo.svg' }, 'the logo must be read from the document');
assert.deepEqual(
  orderNavigation(parsed.navigation).map((item) => item.id),
  ['first', 'later'],
  'navigation must render in ascending order',
);
assert.deepEqual(
  groupFooterLinks(parsed.footerLinks).map((group) => group.group),
  ['Partners', 'Legal'],
  'footer links must be grouped by group',
);
assert.equal(groupFooterLinks(parsed.footerLinks)[0].links.length, 2);
assert.match(theme, /src=\{branding\.logo\.src\}/, 'the logo must render from branding.json');
assert.match(theme, /orderNavigation\(branding\.navigation\)/, 'navigation must render from branding.json');
assert.match(theme, /groupFooterLinks\(branding\.footerLinks\)/, 'footer links must render from branding.json');
assert.match(theme, /aria-label="Primary"/, 'the navigation must be programmatically named');

assert.deepEqual(parseBranding({ ...valid, logo: { kind: 'data', src: 'data:image/svg+xml;base64,PHN2Zy8+' } }).logo, {
  kind: 'data',
  src: 'data:image/svg+xml;base64,PHN2Zy8+',
});
assert.throws(() => parseBranding({ ...valid, name: undefined }), BrandingError, 'a missing name must be rejected');
assert.throws(() => parseBranding({ ...valid, theme: 'sepia' }), BrandingError, 'an unknown theme must be rejected');
assert.throws(
  () => parseBranding({ ...valid, navigation: [{ id: 'a', label: 'A', href: '/a' }] }),
  BrandingError,
  'a navigation entry without order must be rejected',
);
assert.throws(
  () => parseBranding({ ...valid, footerLinks: [{ id: 'a', label: 'A', href: '/a' }] }),
  BrandingError,
  'a footer link without group must be rejected',
);
assert.throws(() => parseBranding({ ...valid, logo: { kind: 'path', src: 'data:image/svg+xml;base64,PHN2Zy8+' } }), BrandingError);
assert.throws(() => parseBranding({ ...valid, locale: '' }), BrandingError, 'an empty locale must be rejected');
try {
  parseBranding({ ...valid, navigation: [{ id: 'a', label: 'A', href: '/a', order: 'first' }] });
  assert.fail('a non-numeric order must be rejected');
} catch (thrown) {
  assert.match(thrown.message, /navigation\[0\]\.order/, 'the failure must name the offending field path');
}

// --- Accessibility invariants ------------------------------------------------

assert.doesNotMatch(styles, /#[0-9a-fA-F]{3,8}\b/, 'the package CSS must contain no color literal');
assert.doesNotMatch(styles, /\brgba?\(/, 'the package CSS must contain no color literal');
assert.doesNotMatch(formsCode, /#[0-9a-fA-F]{6}\b/, 'the TSX must contain no color literal');
assert.doesNotMatch(themeCode, /#[0-9a-fA-F]{6}\b/, 'the TSX must contain no color literal');
assert.match(styles, /@import "\.\.\/design-tokens\/dist\/tokens\.css"/, 'the stylesheet must load the token layer');
assert.match(styles, /:focus-visible \{[\s\S]*outline: 2px solid var\(--focus-ring\)/, 'focus must be visible');
assert.match(styles, /@media \(prefers-reduced-motion: reduce\)/, 'reduced motion must be honoured');
assert.match(styles, /min-height: 24px; min-width: 24px/, 'pointer targets must be at least 24 by 24 CSS px');
assert.match(styles, /\.platform-auth-sticky \{ position: sticky; top: 0; background: var\(--color-background\)/, 'the focus indicator must not be obscured');
assert.match(styles, /#main-content \{ scroll-margin-top: var\(--space-8\); \}/, 'following the skip link must not park the target under the sticky header');
assert.match(styles, /@media \(max-width: 30rem\)/, 'the page must reflow at 320 CSS px');
assert.match(styles, /box-sizing: border-box/, 'a control must not overflow its container at 320 CSS px');
assert.doesNotMatch(styles, /\.visually-hidden\s*\{/, 'the canonical visually-hidden primitive must not be redefined');

// Programmatic names and descriptions: every control the auth pages compose gets
// them from the shared primitives, so the guarantee is asserted on those too.
assert.match(primitives, /<label htmlFor=\{fieldId\}>\{label\}<input/, 'Field must bind a label to its input');
assert.match(primitives, /aria-describedby=\{error \|\| helper \? messageId : undefined\}/, 'a hint or error must be programmatically associated');
assert.match(primitives, /aria-invalid=\{error \? true : undefined\}/, 'an invalid field must expose aria-invalid');
assert.match(primitives, /role=\{role\}/, 'State must expose a role');
assert.match(theme, /<a className="visually-hidden platform-auth-skip" href="#main-content">/, 'the skip link must be the first focusable element and target the main region');

// --- Contrast: measured from the token values, not asserted on names ----------

// The values come out of ui/tokens/tokens.json through the same walk and alias
// resolver the generator and verify-ui.mjs use, so this test cannot disagree with
// what is actually emitted. The dark palette is the theme override map, applied the
// way generate-tokens.mjs applies it. No color value is written in this test.
const declaredTokens = resolveAliases(readTokens(tokenSource));
const byPath = new Map(declaredTokens.map((token) => [token.path, token]));

function palette(mode) {
  const overrides = new Map();
  for (const block of readThemeOverrides(tokenSource)) {
    if (block.mode !== mode) continue;
    for (const group of block.groups) {
      for (const entry of group.entries) {
        overrides.set(`semantic.${group.group}.${entry.key}`, entry.alias);
      }
    }
  }
  const values = new Map();
  for (const token of declaredTokens) {
    const alias = overrides.get(token.path);
    values.set(token.name, (alias ? byPath.get(alias) : token).literal);
  }
  return values;
}

const LIGHT = palette('light');
const DARK = palette('dark');

/** WCAG 2.x relative luminance of a #rgb or #rrggbb value. */
function relativeLuminance(value) {
  const digits = String(value).replace('#', '');
  const channels = digits.length === 3 ? [...digits].map((c) => c + c) : digits.match(/../g);
  assert.ok(channels && (channels.length === 3), `${value} is not a colour the WCAG formula accepts`);
  const [r, g, b] = channels.map((pair) => {
    const channel = Number.parseInt(pair, 16) / 255;
    return channel <= 0.03928 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrastRatio(foreground, background) {
  const a = relativeLuminance(foreground);
  const b = relativeLuminance(background);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

function token(mode, name) {
  const value = mode.get(name);
  assert.ok(value, `${name} is not emitted from ui/tokens/tokens.json, so no pair can be measured against it`);
  return value;
}

// Every foreground/background pair the auth pages actually draw, with the WCAG
// 2.2 AA minimum that applies to it.
const CONTRAST_PAIRS = [
  ['--color-foreground', '--color-background', 4.5, 'body text on the page'],
  ['--color-foreground', '--color-surface', 4.5, 'text and field labels inside the card'],
  ['--color-foreground', '--color-muted', 4.5, 'a disabled control label'],
  ['--color-muted-foreground', '--color-background', 4.5, 'the form description'],
  ['--color-muted-foreground', '--color-surface', 4.5, 'a State message inside the card'],
  ['--color-muted-foreground', '--color-muted', 4.5, 'a disabled submit control'],
  ['--color-primary', '--color-background', 4.5, 'a navigation link on the page'],
  ['--color-primary', '--color-surface', 4.5, 'a footer link on a surface'],
  ['--color-on-primary', '--color-primary', 4.5, 'the submit control label'],
  ['--color-on-primary', '--color-primary-hover', 4.5, 'the submit control label on hover'],
  ['--color-danger', '--color-surface', 4.5, 'the error alert inside the card'],
  ['--color-success', '--color-surface', 4.5, 'the success alert inside the card'],
  ['--color-warning', '--color-surface', 4.5, 'the warning alert inside the card'],
  ['--focus-ring', '--color-background', 3, 'the focus indicator on the page'],
  ['--focus-ring', '--color-surface', 3, 'the focus indicator inside the card'],
  ['--color-border', '--color-surface', 3, 'the field edge that identifies the control'],
  ['--color-border', '--color-background', 3, 'the card edge and the footer divider'],
];

const measurements = [];
for (const [foreground, background, minimum, where] of CONTRAST_PAIRS) {
  for (const [mode, paletteMap] of [['light', LIGHT], ['dark', DARK]]) {
    const from = token(paletteMap, foreground);
    const to = token(paletteMap, background);
    const ratio = contrastRatio(from, to);
    measurements.push({ mode, where, foreground, background, from, to, ratio, minimum });
    assert.ok(
      ratio >= minimum,
      `${mode} ${where}: ${foreground} (${from}) on ${background} (${to}) measures ${ratio.toFixed(2)}:1, below the ${minimum}:1 minimum`,
    );
  }
}

// No colour may reach the auth pages unmeasured: every colour custom property the
// stylesheet names has to be a foreground or a background in the table above.
const measured = new Set(CONTRAST_PAIRS.flatMap(([foreground, background]) => [foreground, background]));
for (const [, name] of styles.matchAll(/var\((--(?:color|focus-ring)[a-z0-9-]*)\)/g)) {
  assert.ok(measured.has(name), `styles.css draws ${name}, which no contrast pair measures`);
}

console.log('Measured WCAG 2.2 AA contrast from ui/tokens/tokens.json:');
for (const row of measurements) {
  console.log(
    `  ${row.mode.padEnd(5)} ${row.ratio.toFixed(2).padStart(6)}:1 (min ${row.minimum})  ${row.foreground} on ${row.background}  ${row.where}`,
  );
}

console.log('React auth component contract tests passed.');
