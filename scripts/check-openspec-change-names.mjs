import { readdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../openspec/changes/', import.meta.url));
const validName = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/;

try {
  const entries = await readdir(root, { withFileTypes: true });
  const invalid = entries
    .filter(entry => entry.isDirectory() && entry.name !== 'archive' && !entry.name.startsWith('.'))
    .map(entry => entry.name)
    .filter(name => !validName.test(name))
    .sort();
  if (invalid.length) {
    for (const name of invalid) console.error(`Invalid active OpenSpec change name: ${name}`);
    process.exitCode = 1;
  } else {
    console.log('check-openspec-change-names: PASS');
  }
} catch (error) {
  console.error(`check-openspec-change-names: unable to read active changes: ${error.message}`);
  process.exitCode = 1;
}
