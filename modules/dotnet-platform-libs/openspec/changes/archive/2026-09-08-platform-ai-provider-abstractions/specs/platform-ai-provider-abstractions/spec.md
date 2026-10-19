# AI provider abstractions

## ADDED Requirements

### Requirement: AI calls SHALL use provider-neutral contracts

Applications SHALL call text, structured-output, streaming, and embedding capabilities without
depending on a provider SDK.

#### Scenario: Provider selection changes

- **WHEN** an application routes a feature from OpenAI to DeepSeek
- **THEN** the application request contract remains unchanged
- **AND** provider-specific unsupported features return an explicit capability result

### Requirement: AI requests SHALL be attributable and bounded

Every request SHALL support a feature key, timeout, cancellation, usage accounting, and a
policy hook for quota, privacy, and redaction.

#### Scenario: Feature quota exceeded

- **WHEN** the feature policy rejects a request before provider execution
- **THEN** no provider call occurs
- **AND** the result identifies the quota reason

### Requirement: AI data SHALL not be logged by default

The platform SHALL exclude prompts, responses, credentials, and sensitive provider payloads from
default logs while retaining correlation, feature, model, latency, and usage metadata.

#### Scenario: Provider error

- **WHEN** a provider returns an error
- **THEN** diagnostics contain a safe classified failure
- **AND** raw prompt/response content is absent unless an explicit development-only sink is configured
