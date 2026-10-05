## ADDED Requirements

### Requirement: Port allocator refusal never terminates an unrelated listener

Forge SHALL allocate a preview port by walking the configured width-wide range
upwards and binding only the candidates the kernel reports as free, SHALL
refuse with `studio-port-unavailable` once the whole range is busy, and SHALL
NOT terminate, close or otherwise disturb any process it did not start. This
invariant SHALL be observable from outside Forge: when the range is occupied,
every occupied port SHALL still accept connections afterwards and SHALL still
refuse to be re-bound.

#### Scenario: A fully occupied range is refused

- **WHEN** every port in the configured range is already bound by processes
  Forge did not start
- **THEN** Forge refuses with the typed `studio-port-unavailable` reason,
  persists the failed preview state, and records no ready state

#### Scenario: The occupied listeners survive the refusal

- **WHEN** a preview start is refused because the range is occupied
- **THEN** every port that was occupied before the attempt still accepts a
  connection and still refuses to be re-bound afterwards, so no unrelated
  listener was killed

#### Scenario: A partly occupied range still starts

- **WHEN** only some ports in the configured range are busy
- **THEN** Forge allocates the first free candidate and starts the preview
