#!/usr/bin/env node
import { cpSync, existsSync, mkdirSync, readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function resolveSource() {
  const envDir = process.env.PLATFORM_CONTRACTS_DIR;
  if (envDir && existsSync(envDir)) return resolve(envDir);
  const sibling = resolve(ROOT, "../platform-contracts");
  if (existsSync(sibling)) return sibling;
  console.error(
    "sync-contracts: cannot resolve platform-contracts source. Set PLATFORM_CONTRACTS_DIR or place a sibling checkout at ../platform-contracts"
  );
  process.exit(1);
}

function resolveForgeVocabularySource() {
  const inTree = resolve(ROOT, "modules/workspace-governance");
  if (existsSync(inTree)) return inTree;
  console.error(
    "sync-contracts: Forge's bundled vocabulary source is missing from its internal modules"
  );
  process.exit(1);
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

const src = resolveSource();
console.log(`sync-contracts: source ${src}`);

const files = [
  "schemas/audit-event.schema.json",
  "schemas/capability.schema.json",
  "schemas/envelope.schema.json",
  "schemas/gate-result.schema.json",
  "schemas/identity-subject.schema.json",
  "schemas/job-outcome.schema.json",
  "schemas/permission-decision.schema.json",
  "schemas/readiness.schema.json",
  "schemas/release-evidence.schema.json",
  "schemas/tenant-context.schema.json",
  "schemas/registry.schema.json",
  "schemas/registry.json",
];

for (const rel of files) {
  const from = join(src, rel);
  if (!existsSync(from)) {
    console.error(`sync-contracts: missing source file ${rel}`);
    process.exit(1);
  }
  try { JSON.parse(readFileSync(from, "utf8")); } catch (e) {
    console.error(`sync-contracts: unparseable ${rel}: ${e.message}`);
    process.exit(1);
  }
}

const dest = join(ROOT, "contracts");
const schemasDir = join(dest, "schemas");
const vocabDir = join(dest, "vocabulary");
mkdirSync(schemasDir, { recursive: true });
mkdirSync(vocabDir, { recursive: true });

for (const rel of files) {
  const from = join(src, rel);
  let to;
  if (rel === "schemas/registry.json") to = join(dest, "registry.json");
  else to = join(dest, rel);
  cpSync(from, to);
  console.log(`  copied ${rel}`);
}

const registry = JSON.parse(readFileSync(join(src, "schemas/registry.json"), "utf8"));
const sfs = registry.secret_field_substrings;
if (!Array.isArray(sfs)) {
  console.error("sync-contracts: registry missing secret_field_substrings");
  process.exit(1);
}
writeFileSync(join(vocabDir, "secret-field-substrings.json"), JSON.stringify(sfs, null, 2) + "\n");
console.log("  wrote vocabulary/secret-field-substrings.json");

let revision = "unknown";
try {
  const { execSync } = await import("node:child_process");
  revision = execSync("git rev-parse HEAD", { cwd: src }).toString().trim();
} catch {}

// Governance vocabulary: consumed verbatim through the same digest-pinned
// mechanism, with its own source and revision. The file is data, never
// executable input; Forge validates values against it and never edits it.
const govSrc = resolveForgeVocabularySource();
console.log("sync-contracts: loading Forge's bundled vocabulary");
const govFile = join(govSrc, "vocabulary.json");
if (!existsSync(govFile)) {
  console.error("sync-contracts: missing source file vocabulary.json");
  process.exit(1);
}
let govDoc;
try { govDoc = JSON.parse(readFileSync(govFile, "utf8")); } catch (e) {
  console.error(`sync-contracts: unparseable vocabulary.json: ${e.message}`);
  process.exit(1);
}
if (govDoc.schema_version !== 1 || !Array.isArray(govDoc.profiles) || !Array.isArray(govDoc.kinds)) {
  console.error("sync-contracts: vocabulary.json must carry schema_version 1 with profiles and kinds arrays");
  process.exit(1);
}
cpSync(govFile, join(vocabDir, "governance-vocabulary.json"));
console.log("  copied vocabulary/governance-vocabulary.json");
let forgeRevision = "unknown";
try {
  const { execSync } = await import("node:child_process");
  forgeRevision = execSync("git rev-parse HEAD", { cwd: ROOT }).toString().trim();
} catch {}

const entries = [];
function collect(dir, prefix) {
  for (const name of readdirSync(dir).sort()) {
    const full = join(dir, name);
    const rel = prefix ? `${prefix}/${name}` : name;
    if (statSync(full).isDirectory()) collect(full, rel);
    else if (name !== "manifest.json") {
      const forgeOwned = rel === "vocabulary/governance-vocabulary.json";
      entries.push({
        path: rel,
        sha256: sha256(full),
        source: forgeOwned ? "forge" : "platform-contracts",
        revision: forgeOwned ? forgeRevision : revision,
      });
    }
  }
}
collect(dest, "");

const manifest = {
  schema_version: 1,
  source: "platform-contracts",
  revision,
  synced_at: new Date().toISOString(),
  files: entries.sort((a, b) => a.path.localeCompare(b.path)),
};
writeFileSync(join(dest, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
console.log(`  wrote manifest.json revision ${revision}`);
