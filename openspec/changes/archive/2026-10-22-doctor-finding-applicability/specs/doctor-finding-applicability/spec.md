## ADDED Requirements

### Requirement: The doctor's findings and controls share one applicability rule

Forge SHALL apply the same applicability rule to a configuration finding
and to the maturity control that mirrors it. A configuration finding SHALL
be applicable when the project's target maturity reaches the level that
declares the mirrored control, and the DriftWatch configuration finding
SHALL also be applicable when the manifest's release checks declare a
DriftWatch check. A finding that does not apply SHALL be reported as not
applicable, SHALL keep its evidence, and SHALL NOT be rendered as a
warning.

#### Scenario: A configuration the project does not require is not applicable

- **WHEN** doctor runs on a project whose target maturity is below the
  level that declares the mirrored control
- **THEN** the configuration finding is reported as not applicable with
  its evidence, and the finding and the mirrored control agree

#### Scenario: A required configuration that is missing still warns

- **WHEN** doctor runs on a project whose target maturity reaches the
  level that declares the mirrored control, and the configuration is
  missing
- **THEN** the configuration finding warns with its evidence and its
  remediation class

#### Scenario: A declared DriftWatch check is required at any maturity

- **WHEN** doctor runs on a project whose manifest declares a DriftWatch
  check in its release checks
- **THEN** the DriftWatch configuration finding is applicable even when
  the target maturity is below L2, and warns when the configuration is
  missing

#### Scenario: The checker plane reports no warning for it

- **WHEN** the checker document is emitted for a project that does not
  require a configuration
- **THEN** no alert is emitted for that configuration finding
