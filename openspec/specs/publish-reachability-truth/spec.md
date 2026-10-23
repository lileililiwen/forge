# publish-reachability-truth Specification

## Purpose
The publish summary states what the publish run established: a healthy
publish names the address the deploy published, and only when the report
carries one, and no surface claims that the project is reachable, because
no stage of a publish observes a reachability.
## Requirements
### Requirement: The publish summary names the address it published

Forge SHALL state in the publish summary only what the publish run
established. A healthy publish SHALL name the address the deploy
published when the report carries one, and SHALL say that no address
was published when the report carries none. The summary SHALL NOT claim
that the project is reachable, because no stage of a publish observes a
reachability.

#### Scenario: A healthy deploy names the published address

- **WHEN** a publish deploy completes and the report carries a subdomain
- **THEN** the summary names the published address, and does not claim that the project is reachable

#### Scenario: A healthy publish without an address says so

- **WHEN** a publish action that publishes no subdomain completes
- **THEN** the summary says that no subdomain was published, and names no address

#### Scenario: One note, every surface

- **WHEN** the publish report is rendered for a human, rendered as JSON, or used as the reason of a publish failure
- **THEN** all three read the same summary
