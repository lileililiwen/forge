## ADDED Requirements

### Requirement: Opt-in realtime transports

The platform SHALL provide independently adoptable SignalR and SSE host adapters without requiring consumers to install both transports or a distributed backplane.

#### Scenario: SSE-only consumer
- **WHEN** an application registers only the SSE adapter
- **THEN** it can map SSE streams without loading SignalR or Redis dependencies

### Requirement: Connection authorization and tenant routing

The adapters SHALL require application-provided authorization and tenant-routing decisions before accepting or broadcasting connection data.

#### Scenario: Cross-tenant broadcast
- **WHEN** a message targets a tenant different from the current connection
- **THEN** the adapter rejects or filters it according to the application routing policy

### Requirement: Bounded lifecycle

The adapters SHALL honor cancellation, connection limits, payload limits, and configured timeouts and SHALL release connection state when clients disconnect.

#### Scenario: Client disconnect
- **WHEN** a client disconnects or its request is cancelled
- **THEN** connection state and resources are released without an unbounded background task

### Requirement: Non-durable delivery disclosure

The adapters SHALL document and expose that realtime delivery is non-durable unless the application supplies its own replay or resynchronization mechanism.

#### Scenario: Reconnect
- **WHEN** a client reconnects after a transport interruption
- **THEN** the application can request resynchronization rather than assuming missed messages were delivered
