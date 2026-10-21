# profile-build-command-truth Specification

## Purpose
Every profile declares the command that builds it, and Forge refuses a
declaration that only analyses. The `flutter-app` profile declared
`flutter analyze` as its build, so `forge build` on a Flutter project
reported a passed build that produced no build output: the analyser reads
the sources, prints a report and exits zero. A profile's build command is
now a command that produces build output — `flutter build bundle` for the
Flutter profile — and a declared build command whose verb is a named
analysis verb is refused with `invalid-profile`, naming the profile, the
command and the verb. The rule is a named list rather than an oracle, so a
verb Forge does not know is accepted, and the catalogue's build commands
are pinned by a repository contract check so a build command cannot change
without that check failing and naming the profile.
## Requirements
### Requirement: A profile's build command builds

Forge SHALL declare a profile's build command as a command that produces
build output, and SHALL refuse a declared build command whose verb is a
named analysis verb. The named verbs SHALL be `analyze`, `lint`,
`clippy`, `fmt`, `format` and `vet`; a verb Forge does not name SHALL be
accepted, because a rule that refused every command it cannot classify
would block legitimate builds. The verb of a build command SHALL be its last
token that is not a flag. The refusal SHALL carry the code
`invalid-profile`, SHALL name the profile, the command and the verb, and
SHALL happen before any file is written.

#### Scenario: A build command that only analyses is refused

- **WHEN** a profile descriptor declares a build command whose verb is a
  named analysis verb
- **THEN** the descriptor is refused with `invalid-profile`, naming the
  profile, the command and the verb, and no file is written

#### Scenario: The refusal reaches the built-in catalogue too

- **WHEN** a built-in profile declares a build command whose verb is a
  named analysis verb
- **THEN** inspecting that profile is refused with `invalid-profile`,
  naming the profile, the command and the verb

#### Scenario: A build command Forge does not recognise is accepted

- **WHEN** a profile descriptor declares a build command whose verb is not
  a named analysis verb
- **THEN** the descriptor is accepted

#### Scenario: The verb is read past the flags

- **WHEN** a profile descriptor declares a build command whose verb is
  followed only by flags
- **THEN** the verb is the command's last non-flag token, and the refusal
  names that verb

### Requirement: The catalogue's build commands are pinned

Forge SHALL pin the declared build command of every supported profile, so
that a build command cannot change in the catalogue without the
repository's contract check failing and naming the profile. Forge SHALL
report the declared build command as the command the build lane runs, so
that the report and the command that ran are the same string.

#### Scenario: The catalogue matches the pinned table

- **WHEN** the repository's contract check reads the profile catalogue
- **THEN** every supported profile's build command is the command the
  table pins, and the check fails naming the profile when one is not

#### Scenario: The build lane runs the declared command

- **WHEN** a project is built
- **THEN** the command that ran is the build command its profile declares
