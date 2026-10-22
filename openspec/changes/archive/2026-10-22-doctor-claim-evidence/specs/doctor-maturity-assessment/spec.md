## MODIFIED Requirements

### Requirement: Doctor finding inventory

Forge SHALL implement forge doctor checking manifest, missing/outdated features, dependency drift, build/deployment configuration, repository, CI, documentation, delivery state and maturity requirements, returning evidence and remediation classification. Forge SHALL judge a project's build configuration against the build definition its profile expects, and SHALL report a check it cannot evaluate as unavailable rather than as a failure of the project. Forge SHALL apply the same applicability rule to a configuration finding and to the maturity control that mirrors it, so that a finding and its control never disagree about whether the project requires the configuration. Forge SHALL state in a finding's evidence and in a control's requirement text only the checks the doctor performed and the facts it read, and SHALL report a fact it cannot know as unknown rather than asserting it.

#### Scenario: Doctor finding inventory success

- **WHEN** a fixture contains known missing configuration and drift
- **THEN** doctor reports stable finding IDs and the corresponding evidence and classifications

#### Scenario: Doctor finding inventory failure

- **WHEN** a required inspector cannot run
- **THEN** doctor reports the check as unavailable and does not label the project healthy

#### Scenario: Doctor finding inventory boundary

- **WHEN** doctor runs on a clean unchanged project twice
- **THEN** findings remain equivalent and project files and remotes remain unchanged

#### Scenario: The delivery state is part of the inventory

- **WHEN** doctor runs for a registered project whose delivery state the delivery lane recorded
- **THEN** the report carries a `delivery-state` finding that names the delivery phase, and the finding is `FAIL` when the delivery failed, `WARN` while the delivery is in progress and `PASS` when it is healthy

#### Scenario: The build configuration is judged per profile

- **WHEN** doctor runs on a project whose profile expects a build definition
- **THEN** the `build-config` finding is `PASS` with the evidence naming the file when that definition is present, and `FAIL` with the evidence naming the expected file when it is missing

#### Scenario: A configuration finding and its control agree

- **WHEN** doctor runs on a project whose target maturity is below the
  level that declares the mirrored control
- **THEN** the configuration finding is reported as not applicable with
  its evidence, and the mirrored control is reported as not applicable
  too

#### Scenario: A statement names the check behind it

- **WHEN** doctor runs on a project and the caller supplies no registry
  observation
- **THEN** the `registry-observation` finding is reported as not applicable
  with evidence naming the unknown, and the `L3-registry` control agrees
  with it
