import { readdir, readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = fileURLToPath(new URL('../', import.meta.url));
const validName = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/;
const failures = [];

function fail(file, line, rule) {
  failures.push(`${file}:${line}: ${rule}`);
}

async function readLines(rel) {
  const text = await readFile(path.join(repo, rel), 'utf8');
  return text.split('\n');
}

async function listMarkdownDocs() {
  const out = [];
  async function walk(rel) {
    const entries = await readdir(path.join(repo, rel), { withFileTypes: true });
    for (const entry of entries) {
      if (entry.name.startsWith('.')) continue;
      const next = path.join(rel, entry.name);
      if (entry.isDirectory()) await walk(next);
      else if (entry.name.endsWith('.md')) out.push(next);
    }
  }
  await walk('docs');
  return out.sort();
}

async function activeChanges() {
  const root = path.join(repo, 'openspec/changes');
  const entries = await readdir(root, { withFileTypes: true });
  return entries
    .filter((e) => e.isDirectory() && e.name !== 'archive' && !e.name.startsWith('.'))
    .map((e) => e.name)
    .sort();
}

// Rule 1: no TBD placeholders in canonical specs or governance docs.
async function checkPlaceholders() {
  const specDirs = await readdir(path.join(repo, 'openspec/specs'));
  const files = specDirs
    .filter((n) => !n.startsWith('.'))
    .map((n) => path.join('openspec/specs', n, 'spec.md'));
  files.push('openspec/config.yaml', 'README.md', 'ROADMAP.md');
  files.push(...(await listMarkdownDocs()));
  for (const file of files) {
    let lines;
    try {
      lines = await readLines(file);
    } catch {
      fail(file, 0, 'placeholder check: file is referenced but unreadable');
      continue;
    }
    lines.forEach((text, i) => {
      if (/^\s*TBD\b/.test(text) || /:\s*`?TBD`?\s*$/.test(text)) {
        fail(file, i + 1, 'canonical/governance placeholder `TBD` must be replaced with source-backed text');
      }
    });
  }
}

// Rule 2: single truthful current_spec pointer in HANDOFF.
async function checkPointer(active) {
  const lines = await readLines('HANDOFF.md');
  const found = [];
  lines.forEach((text, i) => {
    const m = text.match(/^`?current_spec:\s*(\S+?)`?\s*$/);
    if (m) found.push({ name: m[1], line: i + 1 });
  });
  if (active.length === 0) {
    for (const p of found) fail('HANDOFF.md', p.line, `stale current_spec pointer \`${p.name}\`: no active changes remain, remove the line`);
    return;
  }
  if (found.length === 0) {
    fail('HANDOFF.md', 0, `missing current_spec pointer: active change(s) ${active.join(', ')} require exactly one \`current_spec: <name>\` line`);
    return;
  }
  if (found.length > 1) {
    for (const p of found.slice(1)) fail('HANDOFF.md', p.line, 'duplicate current_spec pointer: keep exactly one active pointer line');
  }
  const p = found[0];
  if (p.name === 'none' || p.name === 'TBD') {
    fail('HANDOFF.md', p.line, `stale current_spec pointer \`${p.name}\`: name the one eligible active change`);
  } else if (!validName.test(p.name)) {
    fail('HANDOFF.md', p.line, `invalid current_spec name \`${p.name}\`: must match ^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$`);
  } else if (!active.includes(p.name)) {
    fail('HANDOFF.md', p.line, `stale current_spec pointer \`${p.name}\`: not an active change in openspec/changes/`);
  }
}

// Rule 3: relative local links resolve.
async function checkLinks() {
  const files = ['README.md', 'ROADMAP.md', 'HANDOFF.md'];
  const specDirs = await readdir(path.join(repo, 'openspec/specs'));
  for (const n of specDirs.filter((x) => !x.startsWith('.'))) {
    files.push(path.join('openspec/specs', n, 'spec.md'));
  }
  files.push(...(await listMarkdownDocs()));
  const pending = [];
  for (const file of files) {
    const lines = await readLines(file);
    const base = path.dirname(path.join(repo, file));
    lines.forEach((text, i) => {
      for (const m of text.matchAll(/\]\(([^)]+)\)/g)) {
        let target = m[1].trim().split(/\s+/)[0].replace(/^<|>$/g, '');
        if (!target || /^(https?:|mailto:|#)/i.test(target) || target.startsWith('/')) continue;
        target = target.split('#')[0].split('?')[0];
        if (!target) continue;
        const abs = path.resolve(base, target);
        const label = m[1].trim();
        pending.push(
          stat(abs).catch(() => {
            fail(file, i + 1, `broken local link \`${label}\` resolves outside the repository or to a missing path`);
          }),
        );
      }
    });
  }
  await Promise.all(pending);
}

// Rule 4: status claims stay evidence-qualified.
async function checkStatus(active) {
  const scoped = ['README.md', 'ROADMAP.md', 'openspec/config.yaml'];
  for (const file of scoped) {
    const lines = await readLines(file);
    lines.forEach((text, i) => {
      if (/planning-only/i.test(text)) {
        fail(file, i + 1, 'stale `planning-only` status: baseline is implemented; qualify evidence instead');
      }
      if (/no active changes/i.test(text) && active.length > 0) {
        fail(file, i + 1, `contradictory status: active change(s) ${active.join(', ')} exist`);
      }
    });
  }
  for (const file of ['README.md', 'ROADMAP.md']) {
    const lines = await readLines(file);
    lines.forEach((text, i) => {
      if (/\b(release|project|repository|baseline)\b[^.\n]{0,60}\b(is|are)\b[^.\n]{0,60}\b(complete|done|ready|verified)\b/i.test(text)) {
        fail(file, i + 1, 'unqualified completion claim: express readiness through gate/matrix evidence, not adjectives');
      }
    });
  }
}

const active = await activeChanges();
await checkPlaceholders();
await checkPointer(active);
await checkLinks();
await checkStatus(active);

if (failures.length) {
  for (const f of failures.sort()) console.error(`spec-governance: ${f}`);
  process.exitCode = 1;
} else {
  console.log('check-spec-governance: PASS');
}
