# Forge — Requirements & Product Design v0.3

## 1. Product Definition

**Forge** is a language-agnostic developer control plane and software assembly platform for managing many heterogeneous projects.

Its purpose is not merely to generate project skeletons.

Forge should progressively become a system that allows a developer to:

> describe software requirements in natural language, map those requirements to verified reusable components, assemble a working project deterministically, manage its lifecycle, and use AI only where deterministic standard components are insufficient.

Forge is intended for a development environment containing many projects across:

- ASP.NET Core / C#
- Rust Web
- React
- Next.js
- Flutter
- Python

Forge must remain independent of any single AI model, IDE, coding agent, cloud provider, framework, or hosting platform.

---

# 2. Core Problem

Current AI-assisted development has several recurring problems:

- AI generates code line by line instead of assembling verified components.
- Similar infrastructure is repeatedly reimplemented.
- Project quality differs significantly between repositories.
- Authentication, administration, logging, deployment, auditing, monitoring and similar capabilities are often incomplete.
- Different projects evolve with inconsistent architecture.
- AI repeatedly edits small sections of code and consumes unnecessary tokens.
- Generated code may look correct but have uncertain maturity.
- Existing projects are difficult to upgrade consistently.
- Deployment, Git operations, publishing and repository distribution are repetitive.
- Different AI agents require duplicated instructions.
- Infrastructure tools are fragmented across multiple standalone projects.

Forge should convert these repeated tasks into reusable, versioned, deterministic platform capabilities.

---

# 3. Core Philosophy

Forge should follow five principles.

## 3.1 Deterministic first, AI second

If a problem can be solved by:

- templates
- packages
- codemods
- configuration
- generators
- migrations
- verified components

then Forge should use those approaches first.

AI should primarily handle:

- requirement interpretation
- component selection
- planning
- glue code
- project-specific business logic
- ambiguous migrations
- specification generation

AI should not repeatedly regenerate mature infrastructure.

---

## 3.2 Generate ordinary source code

Forge must not behave like a proprietary low-code runtime.

Generated projects should remain normal projects.

Examples:

```text
dotnet build
cargo build
npm run build
flutter build
python ...
```

A generated project must remain usable even if Forge is removed.

Desired property:

> Strong constraints during generation, weak coupling after generation.

---

## 3.3 Reuse at multiple levels of granularity

Forge should support reusable software assets from large to small:

```text
Profile
↓
Feature
↓
Pattern
↓
Capability
```

Example:

```text
Profile:
aspnet-saas

Feature:
user-management

Pattern:
crud-page

Capabilities:
pagination
validation
authorization
audit
toast
```

Forge should not force every reuse unit to have the same size.

---

## 3.4 Infrastructure evolves gradually

Forge should not require every experimental project to contain production-grade infrastructure.

Projects should support maturity levels.

Example:

```text
L0 Prototype
L1 Standard
L2 Managed
L3 Platform Integrated
L4 Production
```

Forge should understand both:

```text
current maturity
target maturity
```

---

## 3.5 One control plane, many implementations

Forge must provide common concepts across languages while allowing stack-specific implementations.

For example:

```text
Capability: auth
```

may map to:

```text
ASP.NET → ASP.NET Identity / OIDC
Rust    → OIDC / Axum integration
Next.js → Auth.js / OIDC
Flutter → OIDC client
Python  → framework-specific auth
```

Forge manages the capability contract.

The stack adapter manages implementation details.

---

# 4. High-Level Architecture

```text
                         Developer
                             │
                    Natural Language / CLI
                             │
                     AI Control Plane
                             │
                  Intent / Planner / Skills
                             │
                            MCP
                             │
                         Forge Core
                             │
 ┌──────────────┬────────────┼──────────────┬──────────────┐
 │              │            │              │              │
Project      Profile      Feature        Policy        Release
Registry     Registry     Registry        Registry      Engine
 │              │            │              │              │
 └──────────────┴────────────┴──────────────┴──────────────┘
                             │
                     Stack Adapters
                             │
          ┌──────────┬───────┼──────┬────────┐
          │          │       │      │        │
       ASP.NET      Rust    Next   Flutter  Python
                             │
                   Existing Systems
                             │
      DriftWatch / OpenCode / GitHub / Content
```

---

# 5. Forge Components

Forge should initially be implemented as a modular monolith.

Recommended logical modules:

```text
Forge
├── Core
├── Registry
├── Profiles
├── Features
├── Components
├── Policies
├── Intent
├── Planner
├── Generator
├── Upgrade
├── Doctor
├── Agent
├── Distribution
├── Release
├── Deployment
├── Integrations
├── MCP
├── CLI
└── API
```

They do not need to become separate repositories.

---

# 6. Project Manifest

Every managed project should contain a Forge manifest.

Recommended filename:

```text
forge.yaml
```

or:

```text
platform.yaml
```

Forge should support schema versioning.

Example:

```yaml
schema: 1

project:
  id: mortality-reflection
  name: Mortality Reflection
  profile: rust-web
  maturity: L2

runtime:
  language: rust
  version: stable

features:
  auth: 2.1
  admin: 1.4
  postgres: 2.0
  telemetry: 1.2
  privacy: 1.0

quality:
  driftwatch: true

ai:
  spec: openspec
  default_agent: opencode

deployment:
  type: docker
  target: home-server-01

distribution:
  primary: github
  mirrors:
    - gitee

docs:
  source_language: en
  translations:
    zh-CN:
      enabled: false
```

The manifest should become the project-level source of truth for Forge-managed infrastructure.

---

# 7. Project Registry

Forge must maintain a registry of known projects.

Each project should track at minimum:

- Project ID
- Name
- Local path
- Git repository
- Primary remote
- Mirror repositories
- Stack
- Profile
- Maturity level
- Forge schema version
- Platform version
- Enabled features
- Deployment target
- Runtime status
- Last commit
- Quality status
- Agent status
- Documentation status

Example:

```text
Project          Stack        Level   Health
------------------------------------------------
driftwatch       rust         L3      OK
gpa-sim          aspnet       L2      WARN
buddhist-site    nextjs       L2      OK
mobile-app       flutter      L1      WARN
quant-system     python       L1      OK
```

---

# 8. Project Import

Forge must support existing projects.

Command:

```bash
forge import ./project
```

Forge should detect:

- language
- framework
- package manager
- database
- Docker usage
- CI configuration
- authentication
- existing Forge-compatible features
- DriftWatch configuration
- Git remotes
- deployment configuration

Example:

```text
Detected:

Language: Rust
Framework: Axum
Database: PostgreSQL
Container: Docker
GitHub: configured
DriftWatch: missing
Forge manifest: missing

Suggested profile:
rust-web

Suggested maturity:
L1
```

Forge should generate a manifest without aggressively modifying the project unless explicitly requested.

---

# 9. Profiles

A Profile represents a common application architecture.

Initial profiles:

```text
aspnet-web
rust-web
nextjs-web
react-web
flutter-app
python-service
```

Possible later profiles:

```text
aspnet-saas
rust-cli
rust-worker
nextjs-content
python-data
python-ai
flutter-client
```

Profiles should define:

- supported capabilities
- default packages
- project structure
- coding conventions
- build commands
- test commands
- deployment defaults
- quality policies

---

# 10. Feature Registry

Features are reusable functional modules.

Initial examples:

```text
auth
admin
postgres
redis
email
storage
audit
telemetry
health-check
rate-limit
background-jobs
search
billing
notifications
i18n
privacy
content
analytics
```

Each feature should define:

```text
ID
Version
Compatibility
Dependencies
Conflicts
Install strategy
Upgrade strategy
Validation rules
Documentation
Tests
```

Example:

```yaml
feature:
  id: audit
  version: 2.1

supports:
  - aspnet-web
  - rust-web

depends:
  - auth

install:
  strategy: package-plus-generator

validate:
  policy:
    - AUDIT-001
    - AUDIT-002
```

---

# 11. Fine-Grained Component Registry

Forge should go beyond large feature modules.

A reusable unit may be a smaller semantic component.

Examples:

```text
RequirePermission
PaginatedQuery
ValidatedForm
AuditAction
SoftDelete
RetryExternalCall
IdempotencyGuard
ApiMutation
LoadingState
ErrorBoundary
ConfirmDialog
EmptyState
Toast
FilePicker
WebhookReceiver
```

A component must satisfy:

- meaningful semantic purpose
- explicit inputs and outputs
- testability
- versionability
- compatibility metadata
- deterministic installation

Forge should avoid meaningless ultra-fine primitives such as:

```text
if
loop
try/catch
string concat
```

These should remain normal programming constructs.

---

# 12. Component Quality Levels

Reusable components should support maturity classification.

Example:

```text
Experimental
Verified
Certified
Deprecated
```

Optional metadata:

```text
usage_count
test_coverage
last_verified
known_issues
supported_profiles
security_review
```

Certified components should be preferred automatically by the planner.

---

# 13. UI Standard Library

Forge should eventually support reusable UI patterns.

Examples:

```text
Login
Register
Forgot Password
Dashboard
CRUD Table
Filter Bar
Form
Settings
Profile
Billing
Empty State
Success Page
Error Page
Modal
Confirm Dialog
File Upload
Navigation
```

Common design rules should include:

- typography
- spacing
- responsive rules
- loading states
- error states
- success states
- form behavior
- accessibility
- navigation patterns

Possible implementations:

```text
React / Next.js
Flutter
Blazor
```

The UI registry should describe semantic patterns, not merely copied HTML.

---

# 14. Project Creation

Basic command:

```bash
forge new myproject
```

Explicit profile:

```bash
forge new myproject --profile rust-web
```

Feature selection:

```bash
forge new myproject \
  --profile rust-web \
  --feature auth \
  --feature admin \
  --feature postgres \
  --feature telemetry
```

Interactive mode should also be supported.

Example:

```text
Application:
[x] Web
[ ] API
[ ] CLI
[ ] Worker

Stack:
[x] Rust
[ ] ASP.NET
[ ] Next.js
[ ] Flutter
[ ] Python

Features:
[x] Auth
[x] Admin
[x] PostgreSQL
[x] Audit
[x] Telemetry
[ ] Billing
[ ] Redis
```

---

# 15. Deterministic Generation

Forge should prefer deterministic generation through:

- templates
- packages
- dependency installation
- codemods
- code generation
- configuration generation
- database migrations

The intended workflow:

```text
Requirement
↓
Capability graph
↓
Component resolution
↓
Deterministic generation
↓
Glue code generation
↓
AI only for unresolved differences
```

Forge should minimize full-project AI generation.

---

# 16. Natural Language Intent Layer

Forge must not map natural language directly to shell commands.

AI should first generate an Intent.

Example:

```yaml
intent: create_project

project:
  name: histora
  profile: rust-web

capabilities:
  required:
    - auth
    - admin
    - postgres
    - i18n
    - content

  forbidden:
    - billing
    - redis

constraints:
  public: true
  deploy: docker
```

The Intent should then be validated before execution.

---

# 17. Intent Validation

Forge should reject incompatible combinations.

Example:

```text
Flutter application
+
server-side PostgreSQL implementation
```

Result:

```text
Invalid capability graph.

Flutter cannot directly provide the server-side postgres capability.

Recommended:
flutter-client + rust-web backend
```

Validation should occur before generation.

---

# 18. Planner

The Planner converts validated Intent into execution steps.

Example:

```text
1. Select rust-web profile
2. Install auth v3
3. Install postgres v2
4. Install admin v1
5. Install telemetry v2
6. Generate manifest
7. Run doctor
8. Run tests
9. Run DriftWatch
```

Plans should be reviewable before execution where appropriate.

---

# 19. Skill Layer

Skills define AI working procedures.

Examples:

```text
create-project
upgrade-project
prepare-release
fix-quality-findings
onboard-existing-project
deploy-project
mirror-repository
translate-docs
```

Example rule:

```text
When creating a project:

1. inspect available profiles
2. resolve required capabilities
3. prefer certified components
4. never hand-write auth when a supported feature exists
5. generate project
6. run doctor
7. run tests
8. run DriftWatch
9. report unresolved gaps
```

Skills are AI SOPs.

They should not contain the actual platform implementation.

---

# 20. MCP Layer

Forge should provide one primary MCP server.

Recommended concept:

```text
forge-mcp
```

Possible tools:

```text
forge.list_projects
forge.inspect_project
forge.list_profiles
forge.list_features
forge.create_project
forge.import_project
forge.add_feature
forge.remove_feature
forge.upgrade_feature
forge.run_doctor
forge.generate_spec
forge.run_agent
forge.run_tests
forge.commit
forge.push
forge.deploy
forge.publish
```

The MCP server should invoke Forge Core or Forge API.

MCP should not duplicate business logic.

---

# 21. AI Agent Runtime

Forge should integrate existing agent execution infrastructure.

Supported agents may include:

```text
OpenCode
Codex
future CLI agents
```

Agent operations:

```text
start
pause
take over
resume
restart
new session
execute spec
run tests
commit
push
```

Forge should integrate the existing PTY-based agent manager rather than rewriting it.

---

# 22. Specification System

Forge should integrate with the existing Spec workflow.

Possible flow:

```text
Project inspection
↓
Capability gap detection
↓
Policy findings
↓
Automatic fix when deterministic
↓
Generate Spec when semantic changes are required
↓
Agent implementation
↓
Test
↓
DriftWatch
↓
Commit
```

Command examples:

```bash
forge spec generate
forge spec generate --project gpa-sim
forge spec run SPEC-102
```

---

# 23. Doctor

Command:

```bash
forge doctor
```

Doctor should inspect:

- manifest validity
- missing features
- outdated features
- dependency drift
- build configuration
- deployment configuration
- repository status
- CI configuration
- documentation status
- maturity-level requirements

Example:

```text
AUTH-001        PASS
ADMIN-002       WARN
PRIVACY-003     FAIL
DEPLOY-001      PASS
DOCS-002        WARN
```

Doctor should separate:

```text
auto-fixable
AI-fixable
manual
```

---

# 24. DriftWatch Integration

DriftWatch remains the quality/policy engine.

Forge should call it rather than replace it.

Possible policy categories:

```text
architecture
security
privacy
dependency
runtime
spec
documentation
deployment
accessibility
```

Language-specific detectors may exist underneath common policies.

Example:

```text
POLICY-AUTH-001
├── dotnet detector
├── rust detector
├── nextjs detector
└── python detector
```

---

# 25. Maturity Model

Projects should have a maturity level.

## L0 — Prototype

Allowed:

- minimal structure
- incomplete tests
- no deployment automation
- incomplete monitoring

Purpose:

rapid experimentation.

---

## L1 — Standard

Required:

- standard structure
- configuration
- database setup
- logging
- health check
- Docker or equivalent build definition

---

## L2 — Managed

Required:

- authentication where applicable
- administration
- CI
- DriftWatch
- basic deployment automation
- audit where applicable

---

## L3 — Platform Integrated

Required:

- central identity compatibility
- observability
- release automation
- project registry integration
- distribution metadata
- upgrade support

---

## L4 — Production

Required where applicable:

- backups
- recovery procedures
- monitoring
- security hardening
- privacy compliance
- secrets management
- production alerting

Projects should not be forced to upgrade unnecessarily.

---

# 26. Upgrade System

One of Forge's most important capabilities should be upgrading existing projects.

Example:

```bash
forge upgrade
forge upgrade --all
forge upgrade privacy
```

Forge should prefer:

```text
package upgrades
config migrations
codemods
schema migrations
component replacements
```

AI should be used only when deterministic upgrades cannot complete the migration.

Example:

```text
Project A
privacy 1.0 → 2.0
automatic

Project B
privacy missing
automatic install

Project C
privacy 1.2 → 2.0
custom checkout conflict
Spec generated
```

---

# 27. Distribution

Forge should support repository distribution.

Example configuration:

```yaml
distribution:
  primary: github

  mirrors:
    - provider: gitee
      enabled: true
```

Primary repository should remain canonical.

Mirror repositories should normally be one-way.

Example:

```text
Local
  ↓
GitHub primary
  ↓
Gitee mirror
```

Future providers:

```text
GitHub
Gitee
GitLab
Codeberg
```

---

# 28. Documentation Translation

Forge should eventually support AI-assisted documentation translation.

Source-of-truth model:

```text
README.md
↓
README.zh-CN.md
```

The translated file is derivative.

Forge should track source hashes.

Example:

```yaml
docs:
  source: README.md

  translations:
    zh-CN:
      path: docs/README.zh-CN.md
      source_hash: abc123
```

If the source changes:

```text
Chinese translation: STALE
```

Forge should support incremental translation rather than translating the entire document repeatedly.

Command:

```bash
forge docs translate zh-CN
```

---

# 29. Release Engine

Command:

```bash
forge release
```

Possible pipeline:

```text
doctor
↓
tests
↓
DriftWatch
↓
version bump
↓
changelog
↓
commit
↓
tag
↓
push
↓
primary repository
↓
mirrors
↓
package registry
↓
container registry
↓
release notes
```

Example configuration:

```yaml
release:
  versioning: semver

  checks:
    - test
    - doctor
    - driftwatch

  repositories:
    - github
    - gitee

  docs:
    translate:
      - zh-CN
```

---

# 30. Deployment

Forge should not initially attempt to become a full Kubernetes platform.

Initial targets should remain simple.

Examples:

```text
Docker Compose
SSH server
local host
home server
VPS
```

Commands:

```bash
forge deploy
forge deploy --project foo
forge deploy --target server-a
```

Deployment should be adapter-based.

---

# 31. Central Identity and Admin SSO

Forge should eventually support centralized admin authentication using standard OIDC.

Desired behavior:

```text
Forge login
↓
Open Site A admin
↓
OIDC redirect
↓
already authenticated
↓
Site A admin
```

Each project should maintain its own session.

Shared cookies across unrelated applications should not be required.

---

# 32. Existing Systems Integration

Existing systems should retain their identities.

Recommended mapping:

```text
DriftWatch
→ Quality / Policy Plane

OpenCode PTY manager
→ Agent Runtime

GitHub analytics project
→ Analytics Plane

Unified content project
→ Content Plane

Codex proposal workflow
→ Spec Generator
```

Forge acts as the common project model and control plane.

It should not rewrite these systems unnecessarily.

---

# 33. Analytics

Forge should eventually show project-level metrics.

Example:

```text
Projects: 24

Development
Agents running       3
Specs queued        14

Quality
Healthy             18
Warnings             5
Failures             1

Deployment
Running             20
Offline              1
Pending              3

Repositories
GitHub stars        428
7-day growth        +31
```

Analytics should be integrated from existing tools where possible.

---

# 34. CLI Requirements

Initial command surface:

```bash
forge new
forge import
forge list
forge inspect
forge doctor
```

Second phase:

```bash
forge feature add
forge feature remove
forge feature upgrade
forge upgrade
forge spec
```

Third phase:

```bash
forge agent
forge test
forge commit
forge push
forge deploy
```

Later:

```bash
forge publish
forge release
forge docs
forge mirror
```

CLI should remain usable independently of the graphical interface.

---

# 35. API

Forge should eventually expose an API to avoid MCP directly invoking complex shell operations.

Potential routes:

```text
GET    /projects
POST   /projects
GET    /projects/{id}

POST   /projects/{id}/doctor
POST   /projects/{id}/features
POST   /projects/{id}/upgrade
POST   /projects/{id}/specs
POST   /projects/{id}/agents
POST   /projects/{id}/deployments
```

The API is optional for the first version.

---

# 36. Portal

A web portal is useful but should not be an early priority.

Potential sections:

```text
Dashboard
Projects
Features
Components
Policies
Specs
Agents
Deployments
Repositories
Documentation
Analytics
Servers
Settings
```

The portal should consume the same Forge Core APIs as CLI and MCP.

---

# 37. Technology Direction

Forge must remain language-neutral at the product level.

A practical implementation option:

```text
Core / CLI:
Rust

Registry:
SQLite initially
PostgreSQL later if required

Manifest:
YAML

MCP:
stdio initially

Portal:
ASP.NET Core / Next.js later

Integrations:
CLI + local process + HTTP APIs
```

Rust is especially suitable for:

- filesystem work
- process management
- CLI
- PTY
- cross-platform binaries
- fast startup
- low runtime dependency

However, technology choice must not leak into Forge's project model.

---

# 38. Recommended Repository Layout

```text
forge/
├── src/
│   ├── core/
│   ├── cli/
│   ├── registry/
│   ├── intent/
│   ├── planner/
│   ├── generator/
│   ├── upgrade/
│   ├── doctor/
│   ├── mcp/
│   └── integrations/
│
├── profiles/
│   ├── aspnet-web/
│   ├── rust-web/
│   ├── nextjs-web/
│   ├── react-web/
│   ├── flutter-app/
│   └── python-service/
│
├── features/
│   ├── auth/
│   ├── admin/
│   ├── postgres/
│   ├── audit/
│   ├── telemetry/
│   ├── privacy/
│   └── deployment/
│
├── components/
│   ├── backend/
│   ├── ui/
│   └── infrastructure/
│
├── policies/
├── skills/
├── schemas/
├── examples/
└── docs/
```

---

# 39. MVP

The first version should deliberately be small.

## MVP 0.1

Implement only:

```text
forge.yaml
Project Registry
Profile Registry
forge import
forge list
forge inspect
forge new
forge doctor
```

Profiles:

```text
aspnet-web
rust-web
nextjs-web
flutter-app
python-service
```

Do not initially build:

- Portal
- complex AI planner
- full MCP orchestration
- automatic translation
- release management
- SSO
- distributed deployment

---

# 40. MVP 0.2

Add:

```text
Feature Registry

forge feature add
forge feature remove
forge feature upgrade

basic codemod support
basic package upgrade support
```

Start extracting real shared capabilities from existing projects.

---

# 41. MVP 0.3

Integrate:

```text
DriftWatch
Spec generation
OpenCode runner
Codex workflow
```

Flow:

```text
doctor
↓
finding
↓
auto-fix OR spec
↓
agent
↓
test
↓
DriftWatch
```

---

# 42. MVP 0.4

Add MCP.

Expose mature Forge operations only.

Example:

```text
create_project
inspect_project
doctor_project
add_feature
generate_spec
run_agent
```

Do not expose unstable internal functions.

---

# 43. MVP 0.5

Add lifecycle automation:

```text
GitHub
Gitee mirror
documentation translation
release
deployment
```

At this point Forge becomes a true project lifecycle platform.

---

# 44. Non-Goals

Forge should not initially become:

- a new programming language
- a full Kubernetes replacement
- a low-code platform
- a generic CI/CD replacement
- another IDE
- another CMS
- another Git hosting service
- an AI model
- a replacement for DriftWatch
- a replacement for OpenCode
- a universal runtime framework

Its responsibility is coordination, standardization and deterministic software assembly.

---

# 45. Long-Term Product Model

Forge should eventually transform software development from:

```text
Requirement
↓
AI writes thousands of lines
↓
Repeated corrections
↓
Uncertain maturity
```

into:

```text
Requirement
↓
Intent
↓
Capability Graph
↓
Certified Components
↓
Deterministic Assembly
↓
AI fills only the gaps
↓
Quality Validation
↓
Release
```

The long-term goal is therefore not:

> AI writes code faster.

The goal is:

> AI chooses and connects mature software parts faster than code can be regenerated from scratch.

---

# 46. Ultimate Development Experience

Example request:

```text
Create a public Rust web application with authentication,
admin panel, PostgreSQL, multilingual content,
audit logging and Docker deployment.

Use my standard UI.
Publish primarily to GitHub.
Mirror it to Gitee.
No billing.
```

Forge flow:

```text
Natural Language
↓
Intent
↓
Validation
↓
rust-web profile
↓
resolve certified components
↓
auth
admin
postgres
i18n
audit
docker
standard-ui
↓
generate
↓
doctor
↓
tests
↓
DriftWatch
↓
GitHub
↓
Gitee mirror
```

AI-generated custom code should ideally be limited to business-specific differences.

---

# 47. Central Strategic Principle

The most valuable long-term asset in Forge is not the CLI.

It is not MCP.

It is not the AI planner.

It is the growing collection of:

```text
Profiles
Features
Patterns
Capabilities
UI Components
Policies
Upgrade Rules
```

Forge is the factory.

The reusable component registry is the inventory of industrial parts.

AI is the planner and assembly coordinator.

The projects are the finished products.

The long-term objective is to make increasingly sophisticated applications possible through increasingly mature standardized parts, while always retaining ordinary source code, portability, ownership and developer control.
