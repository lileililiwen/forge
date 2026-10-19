## 1. Package and API

- [x] 1.1 Create the optional ASP.NET Core composition package and project references.
- [x] 1.2 Define the module contract, registration options, duplicate validation, and ordered DI registry.
- [x] 1.3 Implement opt-in middleware and endpoint mapping extensions without static state.

## 2. Verification and documentation

- [x] 2.1 Add tests for explicit registration, ordering, duplicate rejection, host isolation, and opt-in mapping.
- [x] 2.2 Add architecture tests proving no Mediator, FluentValidation, EF Core, or product dependency.
- [x] 2.3 Document composition boundaries and compare the deliberate differences from the starter loader.
- [x] 2.4 Run focused tests, build, strict OpenSpec validation, and `git diff --check`.
