# portal-web-ui (delta)

## ADDED Requirements

### Requirement: GitHub metadata views in portal and web

The repositories portal controls and the Delivery web view SHALL expose GitHub metadata observe/propose/create guidance with an accessible topics list, a propose form with a confirm-token field, and a create flow with visibility radios and a push-source checkbox, reusing existing portal/web patterns with no new framework.

#### Scenario: Topics list with dev highlight and text alternative

- **WHEN** the Delivery GitHub metadata card renders topics
- **THEN** the topics appear as a real `<ul>` with dev-prefixed topics (`dev-`/`forge-`, case-insensitive) highlighted in addition to text, and a text alternative names the count and values

#### Scenario: Propose form carries a confirm token field

- **WHEN** the operator opens the propose form
- **THEN** repository, field, value, pull-request/direct mode radios, and a masked confirm-token field are labelled, keyboard operable, validated with an error summary receiving focus, and results announce via a `role="status"` region

#### Scenario: Create flow carries visibility radios and push-source checkbox

- **WHEN** the operator opens the create flow
- **THEN** project, repository, private/public visibility radios, push-source and register-if-missing checkboxes, and an explicit action confirm are labelled and keyboard operable, and the previewed CLI string is exact

#### Scenario: Portal names the github commands

- **WHEN** `forge portal view repositories` renders
- **THEN** `controls_available` names the observe, direct topic propose, and create (with `--register-if-missing`) commands
