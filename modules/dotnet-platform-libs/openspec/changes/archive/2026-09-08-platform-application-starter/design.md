# Design

`Platform.Starter` is a composition package over independently versioned capability packages.
It uses explicit `PlatformApplicationOptions` flags, `TryAdd` registrations, startup option
validation, and documented middleware ordering. It must not hide provider selection or replace
consumer registrations unexpectedly.

The template generates a minimal ASP.NET Core host, application/domain/infrastructure test
projects, configuration files with safe development defaults, health endpoints, a sample
permission, a sample admin route, and a React or Razor client option. A provider remains fake or
unconfigured until the consumer opts into a concrete adapter.

Include a small sample application that exercises identity, permission authorization, admin
user/role listing, billing fake entitlements, AI fake generation, mail/SMS fakes, and shared UI.
This sample is the conformance fixture for future platform changes.
