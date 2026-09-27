# Tasks: forge-independent-project-inventory-fleet

## 1. BFS — Baseline and impact coverage

- [ ] B1. Inventory the 77 workspace entries, existing 22 Compose projects,
  missing 54 Compose projects, profiles, runtime classes, and public ports.
- [ ] B2. Add the `forge-project-inventory/0.1.0` fixtures and map every
  requirement to Forge, provider, project, routing, and Cloudflare evidence.
- [ ] B3. Confirm local inventory is standalone and workspace-governance is an
  optional external adapter with no fixed path.

## 2. DFS — Requirement-by-requirement implementation

- [ ] D1. Implement typed inventory validation and local file provider.
- [ ] D2. Implement bounded external inventory adapter invocation.
- [ ] D3. Convert the compatibility seven-project handoff through the adapter
  and change fleet publishing to normalized snapshots.
- [ ] D4. Add explicit `compose_ready`, `compose_missing`, `invalid`, and
  `source_unavailable` results without silent filtering.
- [ ] D5. Add runtime class and public HTTP port declarations.
- [ ] D6. Update Mac provider and Caddy renderer to route only public HTTP
  services while retaining private containers.
- [ ] D7. Add project-level Compose/Dockerfile contracts for every missing
  runtime, owned by each project repository.

## 3. BFS — Cross-surface regression and completeness

- [ ] R1. Prove Forge works with only a local inventory file.
- [ ] R2. Prove workspace-governance relocation does not require Forge code or
  fixed paths to change.
- [ ] R3. Prove all 77 entries are reported and no entry is silently omitted.
- [ ] R4. Prove web, worker, job, and library containers run on Mac; only
  public HTTP services receive subdomains.
- [ ] R5. Prove database/Redis/Jenkins/private worker ports never reach Caddy
  or Cloudflare.

## 4. Verification

- [ ] V1. Run Forge focused and all applicable Cargo tests.
- [ ] V2. Run Jenkins provider and Caddy renderer tests.
- [ ] V3. Run OpenSpec name and strict validation checks.
- [ ] V4. Capture Mac Docker, port registry, Caddy, Cloudflare, and per-project
  Forge status evidence before claiming fleet completion.
