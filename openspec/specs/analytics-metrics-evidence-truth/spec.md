# analytics-metrics-evidence-truth Specification

## Purpose
The analytics metrics report states only what Forge measured: one rule in the one place the aggregate is built turns a metric whose source was not gathered into an explicit `unavailable` gap with a note, so the report never fabricates a zero, its completion flag never certifies a measurement that was not made, and every surface reads the same rule.
## Requirements
### Requirement: The metrics report states only what Forge measured

Forge SHALL aggregate a project metric only when Forge observed it. A
metric whose source was not gathered SHALL be reported as
`unavailable`, with a note naming the missing evidence, and SHALL NOT
be reported as a value with a source and an observation timestamp. The
doctor-sourced metrics — the quality, agent-session, queued-spec and
deployment counters — SHALL follow that rule, and the summary's
completion flag SHALL be true only when every metric it carries was
measured. The report SHALL NOT fabricate a zero for a metric nobody
answered.

#### Scenario: A metric nobody measured is unavailable

- **WHEN** a metrics report is aggregated without a gathered doctor summary
- **THEN** every doctor-sourced metric is `unavailable`, carries no snapshot and no current value, and names the missing evidence

#### Scenario: The report never fabricates a zero

- **WHEN** a metrics report is aggregated without a gathered doctor summary
- **THEN** no metric carries a source and an observation timestamp for a measurement that was not made

#### Scenario: The completion flag follows the evidence

- **WHEN** a metrics report carries a metric that was not measured
- **THEN** the report is not complete

#### Scenario: A gathered summary is still reported

- **WHEN** a metrics report is aggregated with a doctor summary that was gathered
- **THEN** the doctor-sourced metrics carry the gathered counts with their source and observation window

#### Scenario: One rule, every surface

- **WHEN** the CLI and the assurance browser both aggregate a metrics report
- **THEN** both read the same rule and neither hands the aggregator an invented summary
