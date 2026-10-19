## 1. Contracts and host package

- [x] 1.1 Reconcile existing identity contracts with the adapter boundary; do not add product user entities or permission constants.
- [x] 1.2 Create optional ASP.NET Core identity host integration and central package references.
- [x] 1.3 Use starter identity/current-user/authorization files as behavior references and port tests to configurable claims and application stores.

## 2. Host implementation

- [x] 2.1 Implement configurable claim projection and current-user accessor/middleware behavior.
- [x] 2.2 Implement permission policy builders and authorization audit integration.
- [x] 2.3 Implement authentication/session/verification registration seams and security configuration validation.
- [x] 2.4 Add optional persistence adapter only where it can remain entity/migration-owned by the application.

## 3. Verification and documentation

- [x] 3.1 Add tests for anonymous/authenticated projection, duplicate claims, permission grant/deny, tenant claims, and redaction.
- [x] 3.2 Add architecture tests keeping identity contracts free of ASP.NET Core/EF Core.
- [x] 3.3 Document migration from starter Identity and rollback/claim compatibility rules.
- [x] 3.4 Run serial tests, `git diff --check`, and strict OpenSpec validation.
