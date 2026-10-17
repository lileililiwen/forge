## ADDED Requirements

### Requirement: Responsive workbench grid contains intrinsic content

The project workbench SHALL keep cards within the viewport at 390 CSS pixels
even when a card contains wide tables or horizontally scrollable lifecycle
steps. Mobile grid tracks SHALL be allowed to shrink below their content's
intrinsic width; bounded horizontal regions may scroll within their own
containers, but SHALL NOT widen the page.

#### Scenario: Workbench with wide evidence and lifecycle rail

- **WHEN** the authenticated workbench renders a project with journal rows
  and all lifecycle steps at 390 CSS pixels
- **THEN** cards fit the viewport, wide evidence remains within its labelled
  scroll region, lifecycle steps remain horizontally scrollable, and the
  document has no horizontal overflow
