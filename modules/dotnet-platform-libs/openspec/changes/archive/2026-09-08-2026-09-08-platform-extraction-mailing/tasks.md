# Tasks

## 1. Project and packaging
- [x] Add `src/Platform.Mailing/Platform.Mailing.csproj` targeting `net8.0` with `<PackageReference>` entries for `Microsoft.Extensions.Options` and `Microsoft.Extensions.DependencyInjection.Abstractions` only.
- [x] Add the new project to `Platform.sln`, `Directory.Build.props` packaging metadata, and `Directory.Packages.props` with version `0.1.0`.
- [x] Set `<Nullable>enable</Nullable>`, `<ImplicitUsings>enable</ImplicitUsings>`, and `<TreatWarningsAsErrors>true</TreatWarningsAsErrors>` in the project file.

## 2. Types and registration
- [x] Move `MailAddress`, `MailAttachment`, `MailMessage`, `MailSendOutcome`, `MailSendResult`, `IMailService`, `MailTemplateId`, `IMailTemplateRenderer`, `RenderedMailTemplate`, and `MailingOptions` into the new package under the `Platform.Mailing` namespace.
- [x] Preserve the documented `MailMessage` construction validation and the `MailTemplateId` implicit conversions.
- [x] Add `AddPlatformMailing` extension and preserve the existing option field names.

## 3. Tests
- [x] Add unit tests for `MailMessage` validation, `MailSendResult` construction, `MailTemplateId` implicit conversions, and the configuration validation.
- [x] Add a TestServer integration test that proves the registration and the documented default options.
- [x] Extend `Platform.Architecture.Tests` to forbid `Platform.Mailing` from referencing ASP.NET Core, EF Core, SendGrid, Mailgun, SMTP, Razor, Liquid, or a VisualFlow project.

## 4. Verification
- [x] Run `dotnet restore Platform.sln`, `dotnet build Platform.sln -c Release --no-restore`, `dotnet test Platform.sln -c Release --no-build --nologo`, and `dotnet pack src/Platform.Mailing/Platform.Mailing.csproj -c Release --no-build --nologo`.
- [x] Inspect the produced `.nupkg` and confirm `<dependencies>` contains only the documented platform contracts and the two `Microsoft.Extensions.*` abstractions.
- [x] Run `openspec validate 2026-09-08-platform-extraction-mailing --strict --type change` and `openspec validate platform-mailing --strict --type spec`.
