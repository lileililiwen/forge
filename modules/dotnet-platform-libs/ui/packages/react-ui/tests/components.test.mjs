import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
const source = await readFile(new URL('../src/components.tsx', import.meta.url), 'utf8');
assert.match(source, /export function Button/);
assert.match(source, /export function Field/);
assert.match(source, /export function State/);
assert.match(source, /aria-busy/);
assert.match(source, /aria-invalid/);
console.log('React UI component contract tests passed.');
