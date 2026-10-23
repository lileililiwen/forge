## ADDED Requirements

### Requirement: The translate verdict names the review status

Forge SHALL derive one aggregate verdict from the outcomes of a docs
translate run, and SHALL publish that verdict in the report, in the
human renderer, in the JSON document and in the command's exit code.
The verdict SHALL be `complete` when every locale is translated or
current and every outcome's review status is `ok`; `needs-review` when
every locale is translated or current and at least one outcome's review
status is `needs-review`; and `partial` when at least one locale
failed. The report's `healthy` field SHALL be true only for
`complete`. A `needs-review` verdict SHALL NOT fail the command: the
translation succeeded and a human must review it, so the exit code
stays 0 and the journal records the operation as `done`.

#### Scenario: Every locale reviewed clean is complete

- **WHEN** every locale is translated or current and every outcome's review status is `ok`
- **THEN** the verdict is `complete`, the report is healthy, and the command exits 0

#### Scenario: A locale that needs review is never complete

- **WHEN** every locale is translated or current and at least one outcome's review status is `needs-review`
- **THEN** the verdict is `needs-review`, the report is not healthy, the summary names the locales that need review, and the command exits 0

#### Scenario: A failed locale is partial

- **WHEN** at least one locale failed
- **THEN** the verdict is `partial`, the report is not healthy, and the command exits 1

#### Scenario: A failure outranks a review

- **WHEN** one locale failed and another needs review
- **THEN** the verdict is `partial`

#### Scenario: The release lane names the review

- **WHEN** the release lane translates a locale whose review status is `needs-review`
- **THEN** the docs stage's note names the review status, and the stage's status is still `delivered`
