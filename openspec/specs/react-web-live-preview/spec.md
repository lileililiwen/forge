# react-web-live-preview Specification

## Purpose
A runnable, pinned Vite+React `react-web` scaffold whose development server binds the Forge Studio-reserved port, together with the native browser-render verification for that bounded preview.
## Requirements
### Requirement: Runnable react-web preview scaffold

`forge new --profile react-web` SHALL emit an ordinary client-rendered React
application whose client is served by a pinned Vite development server.
`npm run dev` SHALL serve the application on the port supplied through the
`FORGE_STUDIO_PORT` environment variable, binding `127.0.0.1` and refusing to
shift to another port. `npm run build` and `npm test` SHALL remain
dependency-free and runnable without a network install.

#### Scenario: Dev server binds the reserved port

- **WHEN** the generated project's pinned dependencies are installed and
  `npm run dev` runs with `FORGE_STUDIO_PORT` set to a free port
- **THEN** the Vite dev server listens on exactly that port and the mounted
  React application renders the scaffold greeting in the DOM

#### Scenario: Configured port is occupied

- **WHEN** `npm run dev` runs with `FORGE_STUDIO_PORT` naming an occupied port
- **THEN** the dev server exits non-zero instead of silently serving on a
  different port

#### Scenario: No reserved port

- **WHEN** `npm run dev` runs without `FORGE_STUDIO_PORT`
- **THEN** the dev server falls back to the documented Vite default port and
  still serves the application

#### Scenario: Offline build and test preserved

- **WHEN** the generated project's dependencies have not been installed
- **THEN** `npm run build` and `npm test` both succeed using only the Node
  toolchain

#### Scenario: Missing dependencies are not reported ready

- **WHEN** the pinned dependencies have not been installed and `npm run dev`
  is invoked
- **THEN** the command exits non-zero without binding the reserved port, so the
  Studio reports the preview failed rather than ready

### Requirement: Native live-preview verification

Forge SHALL provide a repeatable native verification that generates the
`react-web` scaffold, installs its pinned toolchain, runs the bounded preview,
and confirms through a real browser engine that the mounted React application
is rendered. When the Node toolchain or a browser engine is unavailable, the
verification SHALL report the check `unverified` and SHALL NOT claim a pass or
a failed render.

#### Scenario: Verified browser render

- **WHEN** the native verification runs on a host with the Node toolchain and
  a browser engine available
- **THEN** it loads the preview URL in the browser engine, observes the
  rendered scaffold greeting in the DOM, and records the check as passed

#### Scenario: Unverified without the toolchain

- **WHEN** the Node toolchain or a browser engine is unavailable
- **THEN** the verification records the check `unverified` and names the
  missing tool instead of reporting a pass or a failed render

