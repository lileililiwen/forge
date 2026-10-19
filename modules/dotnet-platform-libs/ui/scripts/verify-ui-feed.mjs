// Verify that a consumer's committed feed agrees with this repository's feed manifest.
//
// The three refusals below are the same three the .NET `forge` precedent
// rejected, and they are the point of the mechanism (design.md section 2):
//   * an absolute path      - the tree stops resolving when it is relocated;
//   * a `..`-escaping path  - it is only resolvable next to a sibling checkout,
//                            which is exactly the dependency this removes;
//   * an environment variable - a CI runner does not carry the developer's shell,
//                            so it fails identically to a hard-coded path.
// The digest is load-bearing rather than the version (D15): the version cannot
// distinguish today's bytes from drifted bytes, the digest can. ui-shared-layer-
// reachability therefore stayed at 0.1.0 and pinned digests; ui-design-guideline-
// adoption later moved the kit to 0.2.0 precisely because it changed a baselined
// token value on purpose — digests and version move together in a contract change.
//
// Usage:
//   node ui/scripts/verify-ui-feed.mjs <consumer-repo> [<consumer-repo> ...]
//   node ui/scripts/verify-ui-feed.mjs --self
//
// Exit codes: 0 agreement, 1 a named diagnostic was reported, 2 invocation error.

import { createHash } from 'node:crypto';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join, dirname, isAbsolute, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const uiRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const feedManifestPath = join(uiRoot, 'feed/feed.json');

const diagnostics = [];

function report(code, message) {
  diagnostics.push({ code, message });
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function sha256(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

/** Refuse a dependency specifier for a platform UI package, naming why. */
function checkSpecifier(repoLabel, specifier, value) {
  if (!value.startsWith('file:')) {
    report(
      'ui-feed-path-invalid',
      `${repoLabel}: \`${specifier}\` resolves as "${value}". The platform UI layer has no registry and no publish (NG6); declare a file: specifier over a committed tarball.`,
    );
    return null;
  }
  const target = value.slice('file:'.length);
  const variable = /\$\{?([A-Za-z_][A-Za-z0-9_]*)\}?/.exec(target);
  if (variable) {
    report(
      'ui-feed-path-invalid',
      `${repoLabel}: \`${specifier}\` is assembled from environment variable \`${variable[0]}\`. A variable a CI runner does not carry fails exactly like a hard-coded path.`,
    );
    return null;
  }
  if (isAbsolute(target) || target.startsWith('~') || /^[A-Za-z]:[\\/]/.test(target)) {
    report('ui-feed-path-invalid', `${repoLabel}: \`${specifier}\` names the machine-specific absolute path "${target}".`);
    return null;
  }
  if (target.split(/[\\/]/).includes('..')) {
    report('ui-feed-path-invalid', `${repoLabel}: \`${specifier}\` escapes the consumer repository via "${target}".`);
    return null;
  }
  return target;
}

function checkConsumer(repo) {
  const label = relative(process.cwd(), repo) || '.';
  const declarationPath = join(repo, 'ui-conformance.json');
  if (!existsSync(declarationPath)) {
    report('ui-conformance-undeclared', `${label}: no ui-conformance.json. A UI-bearing repository declares consumes-feed or exempt (and a repository with no UI signal needs neither).`);
    return;
  }

  const declaration = readJson(declarationPath);

  if (declaration.state === 'exempt') {
    for (const field of ['reason', 'owner', 'reviewBy']) {
      if (typeof declaration[field] !== 'string' || declaration[field].trim() === '') {
        report('ui-conformance-invalid', `${label}: exempt declaration is missing a non-empty "${field}". An exemption with no reason, owner or review date is not conformant, it is invisible.`);
      }
    }
    return;
  }

  if (declaration.state !== 'consumes-feed') {
    report('ui-conformance-invalid', `${label}: ui-conformance.json declares state "${declaration.state}"; expected "consumes-feed" or "exempt".`);
    return;
  }

  if (declaration.kitVersion !== feed.kitVersion) {
    report('ui-feed-version-mismatch', `${label}: declares kitVersion ${declaration.kitVersion}, feed manifest records ${feed.kitVersion}.`);
  }

  const feedDir = join(repo, declaration.feed?.path ?? '');
  if (isAbsolute(declaration.feed?.path ?? '') || (declaration.feed?.path ?? '').split('/').includes('..')) {
    report('ui-feed-path-invalid', `${label}: feed path "${declaration.feed?.path}" must be relative to the repository root and must not escape it.`);
    return;
  }

  const declared = new Map((declaration.feed?.packages ?? []).map((entry) => [entry.name, entry]));

  for (const expected of feed.packages) {
    const entry = declared.get(expected.name);
    if (!entry) continue; // a package the consumer does not need is not a failure
    if (entry.version !== expected.version) {
      report('ui-feed-version-mismatch', `${label}: ${expected.name} declared at ${entry.version}, feed manifest records ${expected.version}.`);
    }
    const path = join(feedDir, entry.file ?? expected.file);
    if (!existsSync(path)) {
      report('ui-feed-incomplete', `${label}: ${expected.name} is declared but ${relative(repo, path)} is not committed.`);
      continue;
    }
    const actual = sha256(path);
    if (actual !== expected.sha256) {
      report('ui-feed-digest-mismatch', `${label}: ${expected.name} committed bytes are sha256 ${actual}, feed manifest records ${expected.sha256}.`);
    }
    if (entry.sha256 && entry.sha256 !== actual) {
      report('ui-feed-digest-mismatch', `${label}: ${expected.name} declares sha256 ${entry.sha256}, committed bytes are ${actual}.`);
    }
  }

  for (const name of declared.keys()) {
    if (!feed.packages.some((entry) => entry.name === name)) {
      report('ui-feed-undeclared', `${label}: declares ${name}, which the feed manifest does not publish.`);
    }
  }

  if (existsSync(feedDir)) {
    for (const file of readdirSync(feedDir).filter((name) => name.endsWith('.tgz')).sort()) {
      const owner = feed.packages.find((entry) => entry.file === file);
      if (!owner) {
        report('ui-feed-undeclared', `${label}: ${file} is committed in the feed directory but the feed manifest publishes no such package.`);
      } else if (!declared.has(owner.name)) {
        report('ui-feed-undeclared', `${label}: ${file} is committed in the feed directory but ${owner.name} is not declared. An undeclared package is a defect, not an implicit upgrade.`);
      }
    }
  }

  // The specifier is where npm actually resolves, so it is checked too.
  const packageJsonPath = join(repo, 'package.json');
  if (existsSync(packageJsonPath)) {
    const manifest = readJson(packageJsonPath);
    for (const section of ['dependencies', 'devDependencies']) {
      for (const [name, value] of Object.entries(manifest[section] ?? {})) {
        if (!feed.packages.some((entry) => entry.name === name)) continue;
        const target = checkSpecifier(label, `${section}.${name}`, String(value));
        if (target === null) continue;
        const path = resolve(repo, target);
        if (!path.startsWith(`${resolve(repo)}${sep}`)) {
          report('ui-feed-path-invalid', `${label}: \`${section}.${name}\` resolves to ${path}, outside the consumer repository.`);
          continue;
        }
        if (!existsSync(path)) {
          report('ui-feed-incomplete', `${label}: \`${section}.${name}\` names ${target}, which is not committed.`);
          continue;
        }
        const expected = feed.packages.find((entry) => entry.name === name);
        const actual = sha256(path);
        if (expected && actual !== expected.sha256) {
          report('ui-feed-digest-mismatch', `${label}: ${name} at ${target} is sha256 ${actual}, feed manifest records ${expected.sha256}.`);
        }
      }
    }
  }
}

function checkSelf() {
  for (const entry of feed.packages) {
    const path = join(uiRoot, 'feed', entry.file);
    if (!existsSync(path)) {
      report('ui-feed-incomplete', `ui/feed/${entry.file} is recorded in feed.json but not committed.`);
      continue;
    }
    const actual = sha256(path);
    if (actual !== entry.sha256) {
      report('ui-feed-digest-mismatch', `ui/feed/${entry.file} is sha256 ${actual}, feed.json records ${entry.sha256}. Re-run node ui/scripts/pack-ui.mjs.`);
    }
  }
}

if (!existsSync(feedManifestPath)) {
  console.error('ui-feed-manifest-missing: run node ui/scripts/pack-ui.mjs to produce ui/feed/feed.json.');
  process.exit(2);
}

const feed = readJson(feedManifestPath);
const args = process.argv.slice(2);
if (args.length === 0) {
  console.error('Usage: node ui/scripts/verify-ui-feed.mjs <consumer-repo> [...] | --self');
  process.exit(2);
}

if (args.includes('--self')) {
  checkSelf();
  if (diagnostics.length === 0) {
    console.log(`ui-feed self-check passed: ${feed.packages.length} committed tarballs match ui/feed/feed.json at kit version ${feed.kitVersion}.`);
    process.exit(0);
  }
} else {
  for (const repo of args.filter((arg) => !arg.startsWith('--'))) {
    if (!existsSync(repo)) {
      console.error(`ui-check-invocation: ${repo} does not exist.`);
      process.exit(2);
    }
    checkConsumer(resolve(repo));
  }
}

for (const { code, message } of diagnostics) console.error(`${code}: ${message}`);
if (diagnostics.length > 0) {
  console.error(`ui-feed verification failed: ${diagnostics.length} diagnostic(s) across ${args.length} target(s).`);
  process.exit(1);
}
console.log(`ui-feed verification passed for ${args.filter((arg) => !arg.startsWith('--')).length} consumer(s) at kit version ${feed.kitVersion}.`);
