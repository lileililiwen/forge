## ADDED Requirements

### Requirement: The import lane knows every supported profile

Forge SHALL know the language and the package manager of every profile the
catalogue lists as supported, and SHALL detect a project shaped like a
supported profile as that profile. A project whose markers match no
supported profile SHALL NOT be assigned one.

#### Scenario: A supported profile's language and package manager are known

- **WHEN** the import lane answers for a profile the catalogue lists as
  supported
- **THEN** it answers with that profile's language and that profile's
  package manager, and never with `unknown`

#### Scenario: A project shaped like a supported profile is detected

- **WHEN** a repository carrying the markers of a supported profile is
  inspected
- **THEN** the proposal names that profile, and names no other profile as
  an alternative

#### Scenario: A project with no markers is not guessed

- **WHEN** a repository carries the markers of no supported profile
- **THEN** the proposal names no profile and its language, framework and
  package manager stay `unknown`

### Requirement: The npm ecosystem is detected by its markers

Forge SHALL detect a `package.json` project as `nextjs-web` when it names
the `next` dependency or holds a `next.config.*` file, and as `react-web`
when it names the `react` dependency, holds a `vite.config.*` file or
holds a root `index.html`. When both sets of markers are present, the
project SHALL be detected as `nextjs-web`, because a Next.js application
depends on React.

#### Scenario: A Next.js project is detected

- **WHEN** a `package.json` project names the `next` dependency or holds a
  `next.config.*` file
- **THEN** the proposal names `nextjs-web`

#### Scenario: A React project is detected

- **WHEN** a `package.json` project names the `react` dependency, holds a
  `vite.config.*` file or holds a root `index.html`, and holds no Next.js
  marker
- **THEN** the proposal names `react-web`

#### Scenario: Both markers mean Next.js

- **WHEN** a `package.json` project holds a Next.js marker and a React
  marker
- **THEN** the proposal names `nextjs-web` and raises no ambiguity

#### Scenario: Neither marker means no profile

- **WHEN** a `package.json` project holds no Next.js marker and no React
  marker
- **THEN** the proposal names no profile

### Requirement: The import lane's profile list is pinned against the catalogue

Forge SHALL pin the list of profiles the import lane knows against the
profiles the catalogue lists as supported, so that the two cannot drift
apart in either direction, and SHALL hold the lane to detecting every
profile it claims to know.

#### Scenario: The lane and the catalogue agree

- **WHEN** the repository's contract check reads the lane's profile list
  and the supported catalogue
- **THEN** both lists hold the same profiles, every supported profile is
  detectable, and the check fails naming the profile when one of these
  does not hold
