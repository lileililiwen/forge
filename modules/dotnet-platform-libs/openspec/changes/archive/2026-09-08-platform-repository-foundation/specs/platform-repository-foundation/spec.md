## ADDED Requirements

### Requirement: Repository SHALL expose focused platform projects

The repository SHALL contain separately buildable projects for core contracts, ASP.NET Core integration, billing contracts, and testing helpers.

#### Scenario: Build the repository from a clean checkout

- **WHEN** a developer restores and builds the solution
- **THEN** all initial platform projects compile without application-specific source code

### Requirement: Package dependencies SHALL be centrally managed

The repository SHALL define package versions centrally and SHALL not require each project to duplicate versions for shared dependencies.

#### Scenario: Add a shared test dependency

- **WHEN** a test project uses a centrally managed package
- **THEN** its project file declares the package without an independent version override

### Requirement: Core package SHALL remain framework-independent

`Platform.Core` and `Platform.Billing.Contracts` SHALL not depend on ASP.NET Core, EF Core, Stripe SDKs, or application projects.

#### Scenario: Inspect project references

- **WHEN** project references and package references are analyzed
- **THEN** dependency direction confirms that pure contracts remain framework-independent
