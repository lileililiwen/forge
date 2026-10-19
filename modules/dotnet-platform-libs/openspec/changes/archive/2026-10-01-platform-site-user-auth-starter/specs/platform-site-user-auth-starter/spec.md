## ADDED Requirements

### Requirement: Optional site-local authentication template

The platform application template SHALL generate site-local end-user authentication only when `EnableSiteUsers` is explicitly enabled.

#### Scenario: Feature disabled

- **WHEN** a project is generated with the default `EnableSiteUsers=false`
- **THEN** the output contains no site-user routes, user schema, Identity UI dependency, or authentication behavior

#### Scenario: Feature enabled

- **WHEN** a compatible ASP.NET Core app is generated with `EnableSiteUsers=true`
- **THEN** the detached app contains application-owned users/persistence, login/logout/recovery pages, and documented bootstrap/configuration commands

### Requirement: Site-local user and permission ownership

Generated sites SHALL own their user records, session configuration, role/permission mapping, and database migrations; unknown permissions SHALL be denied.

#### Scenario: Site data isolation

- **WHEN** two generated sites enable user authentication
- **THEN** each site stores and manages its own users and permissions without a shared account database

#### Scenario: Unknown permission

- **WHEN** an authenticated user requests an action with no matching permission grant
- **THEN** the generated policy denies the action

### Requirement: Environment-safe bootstrap and sign-in

The generated auth flow SHALL avoid default credentials and user enumeration and SHALL prevent development-only bootstrap behavior in Production.

#### Scenario: Owner bootstrap in Development

- **WHEN** the owner runs the explicit Development bootstrap command
- **THEN** the app creates a password hash and displays a generated random password once without persisting plaintext

#### Scenario: Bootstrap in Production

- **WHEN** the Development bootstrap command or fake auth mode is requested in Production
- **THEN** the app refuses before creating or authenticating a user

#### Scenario: Unknown account sign-in

- **WHEN** an unknown email or incorrect password is submitted
- **THEN** the public response does not reveal which condition occurred
