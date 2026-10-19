# platform-ui-design-system Specification Delta

## MODIFIED Requirements

### Requirement: Shared components SHALL expose complete interaction states

UI primitives SHALL define accessible keyboard, focus, disabled, loading, error, empty,
and forbidden behavior where applicable. Per the adopted brand guideline, every component
guidance SHALL additionally define, and the shipped guidance document SHALL enumerate, the
full visible state set: default, hover, focus-visible, active, disabled, loading, and
error, with the focus-visible state carrying a contrast-verified focus indicator drawn
from the documented focus-ring token. A component or component guidance that omits one of
these states SHALL NOT be considered complete.

#### Scenario: Form submission is pending

- **WHEN** a form action is submitted
- **THEN** the shared component exposes a pending state, prevents accidental duplicate
  submission, and preserves accessible status information

#### Scenario: Component guidance omits a required state

- **WHEN** component guidance does not define one of default, hover, focus-visible,
  active, disabled, loading, or error for a state where it applies
- **THEN** the guidance is reported incomplete against the kit's accessibility rules
  documented in `docs/ui-tokens.md`

#### Scenario: Focus leaves the pointer path

- **WHEN** a user navigates a shared component by keyboard
- **THEN** the component shows a visible `:focus-visible` indicator using the documented
  focus-ring token
- **AND** the indicator meets the recorded non-text contrast threshold in both themes
