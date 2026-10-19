#!/usr/bin/env node
import { createHash } from "node:crypto";
import { readdirSync, readFileSync, lstatSync, writeFileSync, existsSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const contracts = join(root, "contracts");
const manifestPath = join(contracts, "manifest.json");
const write = process.argv.includes("--write-manifest");

function fail(message) {
  console.error(`contracts: ${message}`);
  process.exit(1);
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function walk(dir) {
  const output = [];
  for (const name of readdirSync(dir).sort()) {
    const path = join(dir, name);
    const info = lstatSync(path);
    if (info.isSymbolicLink()) fail(`symlink refused: ${relative(contracts, path)}`);
    if (info.isDirectory()) output.push(...walk(path));
    else if (info.isFile() && path === manifestPath) continue;
    else if (info.isFile()) output.push(path);
    else fail(`unsupported contract asset: ${relative(contracts, path)}`);
  }
  return output;
}

if (!existsSync(contracts)) fail(`contract directory is missing: ${contracts}`);
const files = walk(contracts).map((path) => {
  const rel = relative(contracts, path).split(sep).join("/");
  return { path: rel, sha256: sha256(readFileSync(path)), source: "forge" };
});
const revisionInput = files.map(({ path, sha256: digest }) => `${path}\t${digest}`).join("\n") + "\n";
const revision = sha256(Buffer.from(revisionInput));
for (const file of files) file.revision = revision;
const expected = { schema_version: 1, source: "forge", revision, files };
const rendered = `${JSON.stringify(expected, null, 2)}\n`;

if (write) {
  writeFileSync(manifestPath, rendered);
  console.log(`contracts: wrote ${files.length} Forge-owned asset digests (${revision})`);
  process.exit(0);
}

let actual;
try {
  actual = readFileSync(manifestPath, "utf8");
} catch (error) {
  fail(`cannot read ${relative(root, manifestPath)}: ${error.message}`);
}
if (actual !== rendered) {
  let previous = {};
  try { previous = JSON.parse(actual); } catch {}
  const previousFiles = new Map((previous.files || []).map((entry) => [entry.path, entry.sha256]));
  const nextFiles = new Map(files.map((entry) => [entry.path, entry.sha256]));
  for (const [path, digest] of nextFiles) {
    if (!previousFiles.has(path)) console.error(`contracts: unlisted asset ${path}`);
    else if (previousFiles.get(path) !== digest) console.error(`contracts: digest mismatch ${path}: expected ${previousFiles.get(path)}, actual ${digest}`);
  }
  for (const path of previousFiles.keys()) {
    if (!nextFiles.has(path)) console.error(`contracts: missing asset ${path}`);
  }
  fail(`manifest is stale or malformed; update it with node scripts/check-contracts.mjs --write-manifest`);
}
console.log(`contracts: PASS (${files.length} Forge-owned assets, revision ${revision})`);
