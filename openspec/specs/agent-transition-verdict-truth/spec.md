# agent-transition-verdict-truth Specification

## Purpose
One rule decides the agent journal state: a run-spec records the supervisor's verdict when the supervisor reported one, a transition the runtime refused or lost is recorded as the state the runtime reported, and a transition the runtime carried out is recorded as done. A refused transition is never done. Every surface that records an agent transition reads it: the command, the MCP tool and the HTTP route.
## Requirements
### Requirement: The agent journal state names the reported outcome

Forge SHALL record an agent transition in the journal with the state the
runtime reported. A run-spec verdict SHALL be the verdict the supervisor
reported. A transition the runtime refused or lost SHALL be journalled as
the state the runtime reported and SHALL NOT be journalled as a completed
action, and the row's detail SHALL name the verdict it saw or say that it
saw none. Every surface that records an agent transition — the command,
the MCP tool and the HTTP route — SHALL read the same rule.

#### Scenario: A refused transition is never done

- **WHEN** a transition ends in a state the runtime reported as
  unsupported or disconnected
- **THEN** the journal row records that state and not a completed
  action

#### Scenario: A run-spec with no reported verdict

- **WHEN** a run-spec completes without a supervisor reporting a
  verdict
- **THEN** the journal row records the state the runtime reported and
  the detail says that no verdict was reported

#### Scenario: A verified supervisor run is unchanged

- **WHEN** a supervisor reports a verified loop
- **THEN** the journal row records the supervisor's verdict and the
  detail names it

#### Scenario: One rule, every agent writer

- **WHEN** the command, the MCP tool and the HTTP route record an agent
  transition
- **THEN** all read the one rule for the state the row records

#### Scenario: The HTTP route names the reported state

- **WHEN** the HTTP route records an agent transition
- **THEN** the row that records the transition's outcome carries the
  state the runtime reported and never a completed action the runtime
  refused

#### Scenario: The route's operation row is the request's lifecycle

- **WHEN** the HTTP route records an agent transition
- **THEN** the operation row the route's own lifecycle helper writes
  records that the request was handled, and the transition's row
  records the transition's state
