## ADDED Requirements

### Requirement: A profile's build command runs with the declared toolchain

Forge SHALL declare a build command that runs with the toolchain the
profile declares, and SHALL NOT declare a build command that needs a
package the profile does not declare. The Node ecosystem's `node` and
`npm` SHALL count as one toolchain, because the repository maps both to
the same ecosystem and gives both the same build target form. The
repository's contract check SHALL generate a project for every supported
profile whose toolchain the host has, run the profile's declared build
command through Forge's own build lane, and SHALL fail, naming the
profile, when the command cannot run. A profile whose toolchain the host
lacks SHALL be skipped by that check and SHALL NOT be reported as
passing.

#### Scenario: The declared build command runs

- **WHEN** a project is generated for a profile whose toolchain the host
  has, and the profile's declared build command is run in it
- **THEN** the command succeeds, and the contract check fails naming the
  profile when it does not

#### Scenario: The build command runs the profile's own toolchain

- **WHEN** the repository's contract check reads a profile's declared
  build command
- **THEN** the command's program is a program the profile's declared
  toolchain provides

#### Scenario: A toolchain the host lacks is skipped

- **WHEN** the host does not have a profile's declared toolchain
- **THEN** the contract check skips that profile and claims nothing
  about it

### Requirement: The profile's build command needs nothing beyond the toolchain

Forge SHALL declare a build command that a stock installation of the
profile's declared toolchain can run, without a network install and
without a package outside that toolchain. The `python-service` profile's
build command SHALL compile the project's own Python sources with the
standard library.

#### Scenario: The Python build compiles the project's sources

- **WHEN** the `python-service` profile declares its build command
- **THEN** the command compiles the project's sources with the standard
  library of the declared toolchain and needs no third-party package

#### Scenario: A build that needs an undeclared package is refused by the check

- **WHEN** a profile declares a build command that cannot run with the
  toolchain it declares
- **THEN** the repository's contract check fails, naming the profile
