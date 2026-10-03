// Pack the three platform UI packages into ui/feed/*.tgz plus ui/feed/feed.json.
//
// Why a committed tarball and not a directory (design.md section 2):
//   * a `file:` specifier over a directory is satisfied by npm with a symlink,
//     so the vendored copy is ordinary editable source and can drift with no
//     manifest change and no digest to compare;
//   * a tarball is a checksummed byte artifact, like the committed .nupkg
//     precedent, so "same kit version" is checkable rather than assumed.
// The consumer side is a `file:` specifier in its own package.json over bytes it
// commits itself. There is no registry, no publish and no credential (NG6).
//
// Determinism (V7) rests on three normalisations, all applied here rather than
// hoped for:
//   1. member order   - members are sorted by their tar path, ordinal, so the
//                       filesystem's readdir order cannot reach the bytes;
//   2. mtime/ownership - mtime is fixed at epoch 0, uid/gid 0, empty uname/
//                       gname, mode 0644, so no build clock or checkout user
//                       can reach the bytes;
//   3. compression   - a single gzip stream at a fixed level over that tar.
// Node's zlib writes a zero MTIME in the gzip header, so no timestamp leaks
// there either. Verify with: two runs, `sha256sum ui/feed/*.tgz`.
//
// Usage: node ui/scripts/pack-ui.mjs [--out <dir>]

import { createHash } from 'node:crypto';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { gzipSync } from 'node:zlib';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const uiRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

const PACKAGES = [
  { directory: 'design-tokens', name: '@platform/design-tokens' },
  { directory: 'react-ui', name: '@platform/react-ui' },
  { directory: 'react-shell', name: '@platform/react-shell' },
];

/** Never packed, whatever the `files` array says. */
const EXCLUDED_DIRECTORIES = new Set(['node_modules', '.git', 'tests']);

const args = process.argv.slice(2);
let outDir = join(uiRoot, 'feed');
for (let i = 0; i < args.length; i += 1) {
  if (args[i] === '--out') {
    outDir = join(process.cwd(), args[i + 1]);
    i += 1;
  } else if (args[i] === '-h' || args[i] === '--help') {
    console.log('Usage: node ui/scripts/pack-ui.mjs [--out <dir>]');
    process.exit(0);
  } else {
    console.error(`pack-ui: unknown argument ${args[i]}`);
    process.exit(1);
  }
}

async function collectFiles(packageRoot, files) {
  const collected = [];
  const walk = async (relative) => {
    const entries = await readdir(join(packageRoot, relative), { withFileTypes: true });
    for (const entry of entries.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))) {
      const childRelative = relative ? `${relative}/${entry.name}` : entry.name;
      if (entry.isDirectory()) {
        if (EXCLUDED_DIRECTORIES.has(entry.name)) continue;
        await walk(childRelative);
      } else if (entry.isFile()) {
        collected.push(childRelative);
      }
    }
  };

  for (const entry of files ?? ['.']) {
    // `files` entries are files or directories; a directory contributes everything under it.
    if (await isDirectory(join(packageRoot, entry))) await walk(entry);
    else collected.push(entry);
  }

  // package.json is always present in a package tarball, whatever `files` says.
  collected.push('package.json');
  return [...new Set(collected.map(normalize))].sort();
}

/** `./src/index.ts` and `src/index.ts` are the same member and must sort as one. */
function normalize(path) {
  return path.replace(/^\.\//, '');
}

async function isDirectory(path) {
  try {
    return (await readdir(path)).length >= 0;
  } catch (error) {
    if (error.code === 'ENOTDIR') return false;
    throw error;
  }
}

function octal(value, width) {
  return value.toString(8).padStart(width, '0');
}

function tarHeader({ name, size }) {
  const header = Buffer.alloc(512);
  header.write(name, 0, 100, 'utf8'); // name, normalised to <100 bytes by the caller
  header.write(octal(0o644, 7), 100, 8, 'ascii'); // mode
  header.write(octal(0, 7), 108, 8, 'ascii'); // uid
  header.write(octal(0, 7), 116, 8, 'ascii'); // gid
  header.write(octal(size, 11), 124, 12, 'ascii'); // size
  header.write(octal(0, 11), 136, 12, 'ascii'); // mtime, fixed at epoch
  header.write('        ', 148, 8, 'ascii'); // checksum placeholder
  header.write('0', 156, 1, 'ascii'); // typeflag: regular file
  header.write('ustar\0', 257, 6, 'ascii');
  header.write('00', 263, 2, 'ascii');
  header.fill(0, 265, 297); // uname, gname: empty, never the checkout user
  header.fill(0, 297, 305); // devmajor, devminor
  header.fill(0, 329, 484); // prefix
  header.fill(0, 500, 512); // padding
  let checksum = 0;
  for (const byte of header) checksum += byte;
  header.write(`${octal(checksum, 6)}\0 `, 148, 8, 'ascii');
  return header;
}

function tarball(members) {
  const blocks = [];
  for (const member of members) {
    const data = Buffer.from(member.data);
    const path = `package/${member.path}`;
    if (Buffer.byteLength(path) > 100) {
      throw new Error(`pack-ui: tar path too long for ustar: ${path}`);
    }
    blocks.push(tarHeader({ name: path, size: data.length }), data);
    const padding = (512 - (data.length % 512)) % 512;
    if (padding > 0) blocks.push(Buffer.alloc(padding));
  }
  blocks.push(Buffer.alloc(1024)); // two zero blocks terminate the archive
  return gzipSync(Buffer.concat(blocks), { level: 9 });
}

const packed = [];
for (const { directory, name } of PACKAGES) {
  const packageRoot = join(uiRoot, 'packages', directory);
  const manifest = JSON.parse(await readFile(join(packageRoot, 'package.json'), 'utf8'));
  if (manifest.name !== name) {
    throw new Error(`pack-ui: ${directory}/package.json declares ${manifest.name}, expected ${name}`);
  }
  const files = await collectFiles(packageRoot, manifest.files);
  const members = [];
  for (const path of files) {
    members.push({ path, data: await readFile(join(packageRoot, path)) });
  }
  const bytes = tarball(members);
  const file = `${manifest.name.replace(/^@/, '').replace('/', '-')}-${manifest.version}.tgz`;
  packed.push({
    directory,
    name: manifest.name,
    version: manifest.version,
    file,
    bytes,
    sha256: createHash('sha256').update(bytes).digest('hex'),
    members: members.map((member) => `package/${member.path}`),
  });
}

const versions = new Set(packed.map((entry) => entry.version));
if (versions.size !== 1) {
  throw new Error(`pack-ui: the kit packages disagree on version (${[...versions].join(', ')}); the feed records one kit version`);
}

await mkdir(outDir, { recursive: true });
for (const entry of packed) {
  await writeFile(join(outDir, entry.file), entry.bytes);
}
await writeFile(
  join(outDir, 'feed.json'),
  `${JSON.stringify(
    {
      schema: 1,
      kitVersion: [...versions][0],
      generatedBy: 'ui/scripts/pack-ui.mjs',
      deterministic: 'members sorted by path; mtime 0; uid/gid 0; empty uname/gname; mode 0644; gzip level 9',
      packages: packed.map(({ name, version, file, sha256, members }) => ({ name, version, file, sha256, members })),
    },
    null,
    2,
  )}\n`,
);

for (const entry of packed) {
  console.log(`pack-ui: ${entry.file}  ${entry.bytes.length} bytes  sha256=${entry.sha256}  members=${entry.members.length}`);
}
console.log(`pack-ui: feed.json records kit version ${[...versions][0]} and ${packed.length} package digests.`);
