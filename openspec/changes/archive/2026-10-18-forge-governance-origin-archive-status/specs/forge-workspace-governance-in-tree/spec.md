## MODIFIED Requirements

### Requirement: Forge maintains the imported project after origin archival

Forge documentation SHALL state that the original Workspace Governance
repository is archived and that `modules/workspace-governance/` is the
maintained copy. Any needed source changes SHALL be made in the Forge copy.
The module's source-size conformance and sibling consumer portability SHALL
remain tracked as Forge work, independent of the completed origin archival.

#### Scenario: A needed source change is found after origin archival

- **WHEN** maintainers need to change Workspace Governance behavior or source
- **THEN** they make and verify the change under
  `modules/workspace-governance/` in Forge
- **AND** Forge does not direct maintenance to the archived source repository
- **AND** source-size conformance and sibling portability remain separate from
  the origin's archive status
