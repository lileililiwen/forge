## MODIFIED Requirements

### Requirement: Delivery next-idea transition journals the loop

Delivery publish success SHALL record a `delivery.next-idea` journal row and render it in `#delivery-next-idea` with the `&step=idea` link plus the maintain refresh shortcut. The row SHALL state the transition the server recorded and SHALL NOT claim a publish the server did not observe; a caller's note SHALL be recorded as the caller's report.

#### Scenario: Publish lands the next idea

- **WHEN** `POST .../publish` (or delivery publish) succeeds
- **THEN** the server records kind `delivery.next-idea` and `#delivery-next-idea` announces it via `role=status` naming the journal operation

#### Scenario: The next-idea row claims no unobserved publish

- **WHEN** the next-idea transition is journalled
- **THEN** the row names the transition the server recorded, and a
  caller's note is kept in it marked as the caller's report
