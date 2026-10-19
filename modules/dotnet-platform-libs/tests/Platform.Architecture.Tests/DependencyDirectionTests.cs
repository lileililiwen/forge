using System.Text.Json;
using System.Xml.Linq;

namespace Platform.Architecture.Tests;

public class DependencyDirectionTests
{
    private static readonly string RepositoryRoot = LocateRepositoryRoot();

    private static readonly string[] ProductionProjects =
    {
        "src/Platform.Core/Platform.Core.csproj",
        "src/Platform.Domain/Platform.Domain.csproj",
        "src/Platform.Persistence.EfCore.Migrator/Platform.Persistence.EfCore.Migrator.csproj",
        "src/Platform.Web.Composition/Platform.Web.Composition.csproj",
        "src/Platform.AspNetCore/Platform.AspNetCore.csproj",
        "src/Platform.Billing.Contracts/Platform.Billing.Contracts.csproj",
        "src/Platform.Eventing/Platform.Eventing.csproj",
        "src/Platform.Eventing.Contracts/Platform.Eventing.Contracts.csproj",
        "src/Platform.Eventing.EfCore/Platform.Eventing.EfCore.csproj",
        "src/Platform.Eventing.RabbitMq/Platform.Eventing.RabbitMq.csproj",
        "src/Platform.Caching/Platform.Caching.csproj",
        "src/Platform.Caching.Hybrid/Platform.Caching.Hybrid.csproj",
        "src/Platform.Caching.Redis/Platform.Caching.Redis.csproj",
        "src/Platform.Storage/Platform.Storage.csproj",
        "src/Platform.Storage.Local/Platform.Storage.Local.csproj",
        "src/Platform.Storage.S3/Platform.Storage.S3.csproj",
        "src/Platform.Quota/Platform.Quota.csproj",
        "src/Platform.Quota.Testing/Platform.Quota.Testing.csproj",
        "src/Platform.Quota.AspNetCore/Platform.Quota.AspNetCore.csproj",
        "src/Platform.Privacy/Platform.Privacy.csproj",
        "src/Platform.Privacy.AspNetCore/Platform.Privacy.AspNetCore.csproj",
        "src/Platform.Idempotency/Platform.Idempotency.csproj",
        "src/Platform.Jobs/Platform.Jobs.csproj",
        "src/Platform.Jobs.Hangfire/Platform.Jobs.Hangfire.csproj",
        "src/Platform.Mailing/Platform.Mailing.csproj",
        "src/Platform.Mailing.Smtp/Platform.Mailing.Smtp.csproj",
        "src/Platform.Mailing.SendGrid/Platform.Mailing.SendGrid.csproj",
        "src/Platform.RateLimiting/Platform.RateLimiting.csproj",
        "src/Platform.Realtime/Platform.Realtime.csproj",
        "src/Platform.Realtime.AspNetCore/Platform.Realtime.AspNetCore.csproj",
        "src/Platform.Web/Platform.Web.csproj",
        "src/Platform.Web.Telemetry/Platform.Web.Telemetry.csproj",
        "src/Platform.Web.Cors/Platform.Web.Cors.csproj",
        "src/Platform.Web.Resilience/Platform.Web.Resilience.csproj",
        "src/Platform.FeatureManagement/Platform.FeatureManagement.csproj",
        "src/Platform.Http.Resilience/Platform.Http.Resilience.csproj",
        "src/Platform.Web.OpenApi/Platform.Web.OpenApi.csproj",
        "src/Platform.Web.Versioning/Platform.Web.Versioning.csproj",
        "src/Platform.Testing/Platform.Testing.csproj",
        "src/Platform.Testing.AspNetCore/Platform.Testing.AspNetCore.csproj",
        "src/Platform.Persistence.EfCore/Platform.Persistence.EfCore.csproj",
        "src/Platform.Persistence.Postgres/Platform.Persistence.Postgres.csproj",
        "src/Platform.Persistence.Postgres.Testing/Platform.Persistence.Postgres.Testing.csproj",
        "src/Platform.Identity.Contracts/Platform.Identity.Contracts.csproj",
        "src/Platform.Authorization/Platform.Authorization.csproj",
        "src/Platform.Identity.AspNetCore/Platform.Identity.AspNetCore.csproj",
        "src/Platform.Identity.EntityFrameworkCore/Platform.Identity.EntityFrameworkCore.csproj",
        "src/Platform.Identity.Testing/Platform.Identity.Testing.csproj",
        "src/Platform.Tenant.Lifecycle.Contracts/Platform.Tenant.Lifecycle.Contracts.csproj",
        "src/Platform.Tenant.Lifecycle/Platform.Tenant.Lifecycle.csproj",
        "src/Platform.Tenant.Lifecycle.Testing/Platform.Tenant.Lifecycle.Testing.csproj",
        "src/Platform.Tenant.Lifecycle.AspNetCore/Platform.Tenant.Lifecycle.AspNetCore.csproj",
        "src/Platform.Admin.Contracts/Platform.Admin.Contracts.csproj",
        "src/Platform.Admin.AspNetCore/Platform.Admin.AspNetCore.csproj",
        "src/Platform.Admin.Testing/Platform.Admin.Testing.csproj",
        "src/Platform.Billing/Platform.Billing.csproj",
        "src/Platform.Billing.Testing/Platform.Billing.Testing.csproj",
        "src/Platform.Billing.Stripe/Platform.Billing.Stripe.csproj",
        "src/Platform.Billing.LemonSqueezy/Platform.Billing.LemonSqueezy.csproj",
        "src/Platform.Ai.Contracts/Platform.Ai.Contracts.csproj",
        "src/Platform.Ai/Platform.Ai.csproj",
        "src/Platform.Ai.Testing/Platform.Ai.Testing.csproj",
        "src/Platform.Ai.OpenAiCompatible/Platform.Ai.OpenAiCompatible.csproj",
        "src/Platform.Ai.Anthropic/Platform.Ai.Anthropic.csproj",
        "src/Platform.Ai.Ollama/Platform.Ai.Ollama.csproj",
        "src/Platform.Webhooks.Contracts/Platform.Webhooks.Contracts.csproj",
        "src/Platform.Webhooks.AspNetCore/Platform.Webhooks.AspNetCore.csproj",
        "src/Platform.Webhooks.EfCore/Platform.Webhooks.EfCore.csproj",
        "src/Platform.Auditing.Contracts/Platform.Auditing.Contracts.csproj",
        "src/Platform.Auditing.AspNetCore/Platform.Auditing.AspNetCore.csproj",
        "src/Platform.Auditing.EfCore/Platform.Auditing.EfCore.csproj",
        "src/Platform.Observability/Platform.Observability.csproj",
        "src/Platform.Persistence.Multitenancy/Platform.Persistence.Multitenancy.csproj",
    };

    private static readonly string[] TestOnlyAssemblyNames =
    {
        "Platform.Architecture.Tests",
        "Platform.Core.Tests",
        "Platform.Domain.Tests",
        "Platform.Template.Tests",
        "Platform.Adoption.Tests",
        "Platform.SampleMatrix.Tests",
        "Platform.Persistence.EfCore.Migrator.Tests",
        "Platform.Web.Composition.Tests",
        "Platform.AspNetCore.Tests",
        "Platform.Billing.Contracts.Tests",
        "Platform.Eventing.Contracts.Tests",
        "Platform.Eventing.EfCore.Tests",
        "Platform.Eventing.RabbitMq.Tests",
        "Platform.Caching.Tests",
        "Platform.Caching.Adapter.Tests",
        "Platform.Storage.Tests",
        "Platform.Quota.Tests",
        "Platform.Quota.AspNetCore.Tests",
        "Platform.Privacy.Tests",
        "Platform.Privacy.AspNetCore.Tests",
        "Platform.Testing.Tests",
        "Platform.Web.Tests",
        "Platform.Web.Edge.Tests",
        "Platform.Web.Versioning.Tests",
        "Platform.Persistence.EfCore.Tests",
        "Platform.Persistence.Postgres.Tests",
        "Platform.Identity.Tests",
        "Platform.Admin.Tests",
        "Platform.Billing.Tests",
        "Platform.Billing.ProviderAdapters.Tests",
        "Platform.Ai.Tests",
        "Platform.Ai.Adapter.Tests",
        "Platform.Webhooks.Tests",
        "Platform.Auditing.Tests",
        "Platform.Observability.Tests",
        "Platform.Persistence.Multitenancy.Tests",
        "Platform.FeatureManagement.Tests",
        "Platform.Http.Resilience.Tests",
        "Platform.Realtime.Tests",
        "Platform.Mailing.ProviderAdapters.Tests",
        "Platform.Jobs.Hangfire.Tests",
        "Platform.Tenant.Lifecycle.Tests",
    };

    private static readonly string[] FrameworkIndependentProjects =
    {
        "src/Platform.Core/Platform.Core.csproj",
        "src/Platform.Billing.Contracts/Platform.Billing.Contracts.csproj",
    };

    private static readonly string[] ForbiddenProductionPackagePrefixes =
    {
        "Microsoft.AspNetCore",
        "Microsoft.EntityFrameworkCore",
        "Stripe",
        "Stripe.net",
    };

    private static readonly string[] ForbiddenJobsPackagePrefixes =
    {
        "Microsoft.AspNetCore",
        "Microsoft.EntityFrameworkCore",
        "Hangfire",
        "Quartz",
    };

    private static readonly string[] ForbiddenJobsProjectSegments =
    {
        "VisualFlow",
    };

    private static readonly string[] ForbiddenMailingPackagePrefixes =
    {
        "Microsoft.AspNetCore",
        "Microsoft.EntityFrameworkCore",
        "SendGrid",
        "Mailgun",
        "Smtp",
        "Razor",
        "Liquid",
    };

    private static readonly string[] ForbiddenMailingProjectSegments =
    {
        "VisualFlow",
    };

    private static readonly string[] ForbiddenEventingPackagePrefixes =
    {
        "Microsoft.AspNetCore",
        "Microsoft.EntityFrameworkCore",
        "RabbitMQ",
    };

    private static readonly string[] ForbiddenEventingProjectSegments =
    {
        "VisualFlow",
    };

    private static readonly string[] ForbiddenIdempotencyPackagePrefixes =
    {
        "Microsoft.AspNetCore",
        "Microsoft.EntityFrameworkCore",
        "StackExchange.Redis",
    };

    private static readonly string[] ForbiddenIdempotencyProjectSegments =
    {
        "VisualFlow",
    };

    private static readonly string[] ForbiddenRateLimitingPackagePrefixes =
    {
        "Microsoft.AspNetCore",
        "Microsoft.EntityFrameworkCore",
        "StackExchange.Redis",
    };

    private static readonly string[] ForbiddenRateLimitingProjectSegments =
    {
        "VisualFlow",
    };

    private static readonly string[] ForbiddenWebhooksContractsPackagePrefixes =
    {
        "Microsoft.AspNetCore",
        "Microsoft.EntityFrameworkCore",
    };

    private static readonly string[] ForbiddenWebhooksAspNetCoreProjectSegments =
    {
        "VisualFlow",
    };

    private static readonly string[] ForbiddenWebCompositionPackagePrefixes =
    {
        "Microsoft.EntityFrameworkCore",
        "Mediator",
        "MediatR",
        "FluentValidation",
        "Hangfire",
        "Quartz",
        "RabbitMQ",
        "MassTransit",
        "StackExchange.Redis",
        "Stripe",
    };

    private static readonly string[] ForbiddenPersistenceEfCorePackagePrefixes =
    {
        "Microsoft.AspNetCore",
        "Npgsql",
        "Stripe",
        "StackExchange.Redis",
        "VisualFlow",
    };

    private static readonly string[] ForbiddenMigratorPackagePrefixes =
    {
        "Microsoft.AspNetCore",
        "Microsoft.EntityFrameworkCore.Sqlite",
        "Npgsql",
        "Stripe",
        "StackExchange.Redis",
        "RabbitMQ",
        "Hangfire",
        "Quartz",
        "Mediator",
        "MediatR",
        "FluentValidation",
        "VisualFlow",
    };

    public static IEnumerable<object[]> ProductionProjectsData() =>
        ProductionProjects.Select(p => new object[] { p });

    [Theory]
    [MemberData(nameof(ProductionProjectsData))]
    public void Production_project_exists(string relativePath)
    {
        var fullPath = Path.Combine(RepositoryRoot, relativePath);
        Assert.True(File.Exists(fullPath), $"Project file not found: {relativePath}");
    }

    [Fact]
    public void Production_projects_are_packable_and_the_sample_is_excluded()
    {
        foreach (var relativePath in ProductionProjects)
        {
            var document = XDocument.Load(Path.Combine(RepositoryRoot, relativePath));
            var explicitValue = document.Descendants("IsPackable").SingleOrDefault()?.Value;
            Assert.NotEqual("false", explicitValue, StringComparer.OrdinalIgnoreCase);
        }

        var sample = XDocument.Load(Path.Combine(RepositoryRoot, "samples/Platform.Starter.Sample/Platform.Starter.Sample.csproj"));
        Assert.Equal("false", sample.Descendants("IsPackable").Single().Value, ignoreCase: true);
    }

    [Fact]
    public void All_samples_are_unpacked()
    {
        var projects = Directory.EnumerateFiles(
            Path.Combine(RepositoryRoot, "samples"),
            "*.csproj",
            SearchOption.AllDirectories);
        Assert.NotEmpty(projects);
        foreach (var project in projects.OrderBy(path => path, StringComparer.Ordinal))
        {
            var document = XDocument.Load(project);
            Assert.Equal(
                "false",
                document.Descendants("IsPackable").Single().Value,
                ignoreCase: true);
        }
    }

    [Fact]
    public void Repository_packaging_metadata_points_to_the_public_repository()
    {
        var props = XDocument.Load(Path.Combine(RepositoryRoot, "Directory.Build.props"));

        Assert.Equal("https://github.com/lileililiwen/forge", props.Descendants("RepositoryUrl").Single().Value);
        Assert.Equal("git", props.Descendants("RepositoryType").Single().Value);
        Assert.Equal("MIT", props.Descendants("PackageLicenseExpression").Single().Value);
        Assert.Equal("README.md", props.Descendants("PackageReadmeFile").Single().Value);
        Assert.Equal("snupkg", props.Descendants("SymbolPackageFormat").Single().Value);
        Assert.Equal("true", props.Descendants("EmbedUntrackedSources").Single().Value, ignoreCase: true);
    }

    [Theory]
    [MemberData(nameof(ProductionProjectsData))]
    public void Production_project_does_not_reference_test_projects(string relativePath)
    {
        var references = ReadProjectReferences(relativePath);
        var leaks = references
            .Intersect(TestOnlyAssemblyNames, StringComparer.OrdinalIgnoreCase)
            .ToArray();

        Assert.True(
            leaks.Length == 0,
            $"{relativePath} must not reference test projects but references: {string.Join(", ", leaks)}");
    }

    [Theory]
    [MemberData(nameof(ProductionProjectsData))]
    public void Production_project_does_not_reference_forbidden_frameworks(string relativePath)
    {
        if (relativePath.Contains("Persistence.EfCore", StringComparison.OrdinalIgnoreCase)
            || relativePath.Contains("Persistence.Postgres", StringComparison.OrdinalIgnoreCase)
            || relativePath.Contains("Persistence.Multitenancy", StringComparison.OrdinalIgnoreCase)
            || relativePath.Contains("Identity.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
            || relativePath.Contains("Eventing.EfCore", StringComparison.OrdinalIgnoreCase)
            || relativePath.Contains("Webhooks.EfCore", StringComparison.OrdinalIgnoreCase)
            || relativePath.Contains("Auditing.EfCore", StringComparison.OrdinalIgnoreCase)
            || relativePath.Contains("Auditing.AspNetCore", StringComparison.OrdinalIgnoreCase)
            || relativePath.Contains("Caching.Hybrid", StringComparison.OrdinalIgnoreCase)
            || relativePath.Contains("Caching.Redis", StringComparison.OrdinalIgnoreCase))
            return;
        if (relativePath.Contains("Billing.Stripe", StringComparison.OrdinalIgnoreCase)
            || relativePath.Contains("Billing.LemonSqueezy", StringComparison.OrdinalIgnoreCase)) return;
        var packages = ReadPackageReferences(relativePath);
        var violations = packages
            .Where(p => ForbiddenProductionPackagePrefixes.Any(prefix =>
                p.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            // Platform.Testing.AspNetCore is a test-only host package: it legitimately
            // references Microsoft.AspNetCore.Mvc.Testing, which is guarded separately
            // by Platform_Testing_AspNetCore_declares_FrameworkReference_for_ASPNET.
            // Microsoft.EntityFrameworkCore stays forbidden for it, so the ORM stays
            // out of a package that four repositories consume.
            .Where(p => !(relativePath.Contains("Platform.Testing.AspNetCore", StringComparison.OrdinalIgnoreCase)
                && p.StartsWith("Microsoft.AspNetCore", StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            $"{relativePath} must not reference forbidden packages but references: {string.Join(", ", violations)}");
    }

    [Fact]
    public void Platform_Persistence_EfCore_only_references_Platform_Core()
    {
        var references = ReadProjectReferences("src/Platform.Persistence.EfCore/Platform.Persistence.EfCore.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Core", StringComparison.OrdinalIgnoreCase),
            "Platform.Persistence.EfCore must reference only Platform.Core but references: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Persistence_EfCore_has_no_provider_specific_project_or_package_references()
    {
        var projectReferences = ReadProjectReferences("src/Platform.Persistence.EfCore/Platform.Persistence.EfCore.csproj");
        var packages = ReadPackageReferences("src/Platform.Persistence.EfCore/Platform.Persistence.EfCore.csproj");
        var violations = packages.Where(package => ForbiddenPersistenceEfCorePackagePrefixes.Any(prefix =>
            package.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))).ToArray();

        Assert.DoesNotContain(projectReferences, reference => reference.Contains("Postgres", StringComparison.OrdinalIgnoreCase));
        Assert.Empty(violations);
    }

    [Fact]
    public void Platform_Persistence_Postgres_only_references_EfCore_persistence()
    {
        var references = ReadProjectReferences("src/Platform.Persistence.Postgres/Platform.Persistence.Postgres.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Persistence.EfCore", StringComparison.OrdinalIgnoreCase),
            "Platform.Persistence.Postgres must reference only Platform.Persistence.EfCore but references: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Persistence_EfCore_Migrator_only_references_EfCore_persistence()
    {
        var references = ReadProjectReferences("src/Platform.Persistence.EfCore.Migrator/Platform.Persistence.EfCore.Migrator.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Persistence.EfCore", StringComparison.OrdinalIgnoreCase),
            "Platform.Persistence.EfCore.Migrator must reference only Platform.Persistence.EfCore but references: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Persistence_EfCore_Migrator_has_no_provider_web_or_messaging_references()
    {
        var packages = ReadPackageReferences("src/Platform.Persistence.EfCore.Migrator/Platform.Persistence.EfCore.Migrator.csproj");
        var violations = packages
            .Where(package => ForbiddenMigratorPackagePrefixes.Any(prefix =>
                package.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();
        Assert.True(
            violations.Length == 0,
            "Platform.Persistence.EfCore.Migrator must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Adoption_core_has_no_package_dependencies()
    {
        Assert.Empty(ReadPackageReferences("src/Platform.Adoption/Platform.Adoption.csproj"));
        Assert.Empty(ReadProjectReferences("src/Platform.Adoption/Platform.Adoption.csproj"));
    }

    [Fact]
    public void Template_pack_project_is_content_only()
    {
        const string relativePath = "templates/Platform.Application.Template/Platform.Application.Template.csproj";
        var path = Path.Combine(RepositoryRoot, relativePath);
        Assert.True(File.Exists(path), "Template pack project must exist at " + path);
        var document = XDocument.Load(path);
        Assert.Equal(
            "Template",
            document.Descendants().FirstOrDefault(e => string.Equals(e.Name.LocalName, "PackageType", StringComparison.Ordinal))?.Value.Trim());
        Assert.Equal(
            "false",
            document.Descendants().FirstOrDefault(e => string.Equals(e.Name.LocalName, "IncludeBuildOutput", StringComparison.Ordinal))?.Value.Trim());
        Assert.Empty(ReadProjectReferences(relativePath));
        Assert.Empty(ReadPackageReferences(relativePath));
        var content = document.Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "Content", StringComparison.Ordinal))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .ToArray();
        Assert.NotEmpty(content);
        var excludes = document.Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "Content", StringComparison.Ordinal))
            .Select(e => e.Attribute("Exclude")?.Value ?? string.Empty);
        var excludeText = string.Join(";", excludes);
        Assert.Contains("bin", excludeText, StringComparison.OrdinalIgnoreCase);
        Assert.Contains("obj", excludeText, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Template_content_is_pinned_detached_and_minimal()
    {
        var contentRoot = Path.Combine(RepositoryRoot, "templates", "platform-application-starter");
        Assert.True(Directory.Exists(contentRoot), "Template content tree must exist at " + contentRoot);

        using var templateJson = JsonDocument.Parse(File.ReadAllText(Path.Combine(contentRoot, ".template.config", "template.json")));
        var template = templateJson.RootElement;
        Assert.Equal("Platform.ApplicationStarter", template.GetProperty("identity").GetString());
        Assert.Equal("platform-app", template.GetProperty("shortName").GetString());
        Assert.Equal("StarterApp", template.GetProperty("sourceName").GetString());
        var symbols = template.GetProperty("symbols");
        Assert.True(symbols.TryGetProperty("IncludeTests", out var includeTests) && includeTests.GetProperty("defaultValue").GetString() == "true");
        Assert.True(symbols.TryGetProperty("EnableIdentity", out var enableIdentity) && enableIdentity.GetProperty("defaultValue").GetString() == "false");
        Assert.True(symbols.TryGetProperty("EnablePersistence", out var enablePersistence) && enablePersistence.GetProperty("defaultValue").GetString() == "false");

        foreach (var project in Directory.EnumerateFiles(contentRoot, "*.csproj", SearchOption.AllDirectories))
        {
            var document = XDocument.Load(project);
            var projectDirectory = Path.GetDirectoryName(project) ?? contentRoot;
            foreach (var reference in document
                         .Descendants()
                         .Where(e => string.Equals(e.Name.LocalName, "ProjectReference", StringComparison.OrdinalIgnoreCase)))
            {
                var include = reference.Attribute("Include")?.Value ?? string.Empty;
                var target = Path.GetFullPath(Path.Combine(projectDirectory, include.Replace('\\', Path.DirectorySeparatorChar)));
                Assert.StartsWith(
                    contentRoot + Path.DirectorySeparatorChar,
                    target,
                    StringComparison.Ordinal);
            }

            foreach (var reference in document
                         .Descendants()
                         .Where(e => string.Equals(e.Name.LocalName, "PackageReference", StringComparison.OrdinalIgnoreCase)))
            {
                var name = reference.Attribute("Include")?.Value ?? string.Empty;
                var version = reference.Attribute("Version")?.Value ?? string.Empty;
                Assert.False(string.IsNullOrWhiteSpace(version), $"{project}: {name} must declare a pinned Version attribute.");
                Assert.False(version.Contains('*') || version.Contains('[') || version.Contains('('), $"{project}: {name} must use an exact version. Saw '{version}'.");
            }
        }

        var forbiddenMarkers = new[] { "Dockerfile", "Aspire", "Terraform", "package.json", "node_modules", "React" };
        foreach (var file in Directory.EnumerateFiles(contentRoot, "*", SearchOption.AllDirectories))
        {
            var content = File.ReadAllText(file);
            foreach (var marker in forbiddenMarkers)
            {
                Assert.DoesNotContain(marker, content, StringComparison.Ordinal);
            }
        }

        Assert.False(Directory.Exists(Path.Combine(contentRoot, "bin")), "Template content must not contain bin output.");
        Assert.False(Directory.Exists(Path.Combine(contentRoot, "obj")), "Template content must not contain obj output.");
    }

    [Theory]
    [InlineData("src/Platform.Core/Platform.Core.csproj")]
    [InlineData("src/Platform.Billing.Contracts/Platform.Billing.Contracts.csproj")]
    public void Framework_independent_project_has_no_project_references(string relativePath)
    {
        var references = ReadProjectReferences(relativePath);

        Assert.True(
            references.Length == 0,
            $"{relativePath} must not reference other projects but references: {string.Join(", ", references)}");
    }

    [Fact]
    public void Platform_AspNetCore_only_references_Platform_Core()
    {
        var references = ReadProjectReferences("src/Platform.AspNetCore/Platform.AspNetCore.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Core", StringComparison.OrdinalIgnoreCase),
            $"Platform.AspNetCore must reference only Platform.Core but references: {string.Join(", ", references)}");
    }

    [Fact]
    public void Platform_Testing_references_at_least_one_public_contract()
    {
        var references = ReadProjectReferences("src/Platform.Testing/Platform.Testing.csproj");

        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Testing_AspNetCore_only_depends_on_public_contracts_and_core()
    {
        var references = ReadProjectReferences("src/Platform.Testing.AspNetCore/Platform.Testing.AspNetCore.csproj");
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        var forbidden = references
            .Where(r => r.Contains(".Testing", StringComparison.OrdinalIgnoreCase)
                && !r.Equals("Platform.Testing.AspNetCore", StringComparison.OrdinalIgnoreCase))
            .ToArray();
        Assert.True(
            forbidden.Length == 0,
            "Platform.Testing.AspNetCore must not reference any other testing-support package. Found: " + string.Join(", ", forbidden));
    }

    [Fact]
    public void Platform_Testing_AspNetCore_declares_FrameworkReference_for_ASPNET()
    {
        var path = Path.Combine(RepositoryRoot, "src/Platform.Testing.AspNetCore/Platform.Testing.AspNetCore.csproj");
        var document = System.Xml.Linq.XDocument.Load(path);
        var frameworkReferences = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "FrameworkReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .Where(v => !string.IsNullOrWhiteSpace(v))
            .ToArray();
        Assert.Contains("Microsoft.AspNetCore.App", frameworkReferences, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Testing_AspNetCore_has_no_orm_test_framework_or_container_reference()
    {
        // The shared web test factory must stay dependency-light: no ORM, no test
        // framework, no mocking framework, no container runtime. Database and schema
        // helpers are deferred, and adding any of these would put an ORM or xUnit
        // into the closure of a package four repositories consume.
        var packages = ReadPackageReferences("src/Platform.Testing.AspNetCore/Platform.Testing.AspNetCore.csproj");
        var forbiddenPrefixes = new[]
        {
            "Microsoft.EntityFrameworkCore",
            "xunit",
            "Moq",
            "NSubstitute",
            "FluentAssertions",
            "Testcontainers",
        };
        var violations = packages
            .Where(package => forbiddenPrefixes.Any(prefix =>
                package.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Testing.AspNetCore must not reference an ORM, test framework, mocking framework or container runtime. Found: "
                + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Persistence_Postgres_Testing_references_only_npgsql_and_testcontainers()
    {
        // The shared Postgres fixture is ORM-neutral by contract: raw SQL over
        // Npgsql plus the Testcontainers lifecycle, no project references, so
        // the caller keeps choosing the migration mechanism.
        Assert.Empty(ReadProjectReferences("src/Platform.Persistence.Postgres.Testing/Platform.Persistence.Postgres.Testing.csproj"));
        Assert.Equal(
            ["Npgsql", "Testcontainers.PostgreSql"],
            ReadPackageReferences("src/Platform.Persistence.Postgres.Testing/Platform.Persistence.Postgres.Testing.csproj"));
    }

    [Fact]
    public void Platform_Persistence_Postgres_Testing_has_no_ef_or_test_framework_reference()
    {
        var packages = ReadPackageReferences("src/Platform.Persistence.Postgres.Testing/Platform.Persistence.Postgres.Testing.csproj");
        var violations = packages
            .Where(package => package.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
                || package.StartsWith("xunit", StringComparison.OrdinalIgnoreCase)
                || package.StartsWith("Microsoft.NET.Test", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Persistence.Postgres.Testing must not reference EF Core or a test framework. Found: "
                + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Persistence_Postgres_Testing_default_image_is_pinned()
    {
        var text = File.ReadAllText(Path.Combine(
            RepositoryRoot,
            "src/Platform.Persistence.Postgres.Testing/PostgresTestServerOptions.cs"));
        Assert.Contains("postgres:", text, StringComparison.Ordinal);
        Assert.DoesNotContain(":latest", text, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Production_projects_do_not_reference_Platform_Testing_toolkit()
    {
        var testingToolkitProjects = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            "Platform.Testing",
            "Platform.Testing.AspNetCore",
        };
        foreach (var project in ProductionProjects)
        {
            if (testingToolkitProjects.Contains(Path.GetFileNameWithoutExtension(project)))
            {
                continue;
            }
            var references = ReadProjectReferences(project);
            var leaks = references
                .Where(r => testingToolkitProjects.Contains(Path.GetFileNameWithoutExtension(r) ?? string.Empty))
                .ToArray();
            Assert.True(
                leaks.Length == 0,
                $"{project} must not reference any testing toolkit project. Found: " + string.Join(", ", leaks));
        }
    }

    [Fact]
    public void Production_projects_do_not_reference_any_testing_only_platform_project()
    {
        // Discover every Platform.*.Testing project under src/ so the rule
        // stays in sync when new testing-only platform packages are added
        // (e.g. Platform.Identity.Testing, Platform.Tenant.Lifecycle.Testing).
        var testingProjects = Directory
            .EnumerateDirectories(Path.Combine(RepositoryRoot, "src"))
            .Where(directory => Path.GetFileName(directory).EndsWith(".Testing", StringComparison.OrdinalIgnoreCase))
            .Select(Path.GetFileName)
            .Where(name => !string.IsNullOrEmpty(name))
            .Cast<string>()
            .ToHashSet(StringComparer.OrdinalIgnoreCase);

        Assert.NotEmpty(testingProjects);

        foreach (var project in ProductionProjects)
        {
            if (testingProjects.Contains(Path.GetFileNameWithoutExtension(project) ?? string.Empty))
            {
                continue;
            }
            var references = ReadProjectReferences(project);
            var leaks = references
                .Where(r => testingProjects.Contains(Path.GetFileNameWithoutExtension(r) ?? string.Empty))
                .ToArray();
            Assert.True(
                leaks.Length == 0,
                $"{project} must not reference any testing-only platform project. Found: " + string.Join(", ", leaks));
        }
    }

    [Fact]
    public void Platform_Core_has_no_package_references()
    {
        var packages = ReadPackageReferences("src/Platform.Core/Platform.Core.csproj");

        Assert.True(
            packages.Length == 0,
            "Platform.Core must remain dependency-light but references: " + string.Join(", ", packages));
    }

    [Fact]
    public void Platform_Billing_Contracts_has_no_package_references()
    {
        var packages = ReadPackageReferences("src/Platform.Billing.Contracts/Platform.Billing.Contracts.csproj");

        Assert.True(
            packages.Length == 0,
            "Platform.Billing.Contracts must remain dependency-light but references: " + string.Join(", ", packages));
    }

    [Theory]
    [InlineData("src/Platform.Identity.Contracts/Platform.Identity.Contracts.csproj")]
    [InlineData("src/Platform.Authorization/Platform.Authorization.csproj")]
    [InlineData("src/Platform.Tenant.Lifecycle.Contracts/Platform.Tenant.Lifecycle.Contracts.csproj")]
    public void Identity_contract_projects_have_no_package_or_project_references(string relativePath)
    {
        Assert.Empty(ReadPackageReferences(relativePath));
        Assert.Empty(ReadProjectReferences(relativePath));
    }

    [Fact]
    public void Platform_Identity_EntityFrameworkCore_references_only_identity_contracts_and_persistence()
    {
        var references = ReadProjectReferences("src/Platform.Identity.EntityFrameworkCore/Platform.Identity.EntityFrameworkCore.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Identity.Contracts", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Persistence.EfCore", references, StringComparer.OrdinalIgnoreCase);
        Assert.DoesNotContain(ReadPackageReferences("src/Platform.Identity.EntityFrameworkCore/Platform.Identity.EntityFrameworkCore.csproj"),
            package => package.StartsWith("Stripe", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Platform_Identity_AspNetCore_references_only_identity_contracts_and_authorization()
    {
        var references = ReadProjectReferences("src/Platform.Identity.AspNetCore/Platform.Identity.AspNetCore.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Identity.Contracts", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Authorization", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Identity_AspNetCore_does_not_reference_forbidden_packages()
    {
        var packages = ReadPackageReferences("src/Platform.Identity.AspNetCore/Platform.Identity.AspNetCore.csproj");
        var violations = packages.Where(package => new[]
        {
            "Microsoft.EntityFrameworkCore",
            "Stripe",
            "OpenIddict",
            "Twilio",
        }.Any(prefix => package.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))).ToArray();
        Assert.Empty(violations);
    }

    [Fact]
    public void Platform_Identity_Testing_references_only_identity_contracts_and_authorization()
    {
        var references = ReadProjectReferences("src/Platform.Identity.Testing/Platform.Identity.Testing.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Identity.Contracts", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Authorization", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Tenant_Lifecycle_Contracts_has_no_package_or_project_references()
    {
        Assert.Empty(ReadPackageReferences("src/Platform.Tenant.Lifecycle.Contracts/Platform.Tenant.Lifecycle.Contracts.csproj"));
        Assert.Empty(ReadProjectReferences("src/Platform.Tenant.Lifecycle.Contracts/Platform.Tenant.Lifecycle.Contracts.csproj"));
    }

    [Fact]
    public void Platform_Tenant_Lifecycle_Testing_references_only_tenant_lifecycle_contracts()
    {
        var references = ReadProjectReferences("src/Platform.Tenant.Lifecycle.Testing/Platform.Tenant.Lifecycle.Testing.csproj");
        Assert.Single(references);
        Assert.Contains("Platform.Tenant.Lifecycle.Contracts", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Tenant_Lifecycle_references_only_tenant_lifecycle_contracts_and_core()
    {
        var references = ReadProjectReferences("src/Platform.Tenant.Lifecycle/Platform.Tenant.Lifecycle.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Tenant.Lifecycle.Contracts", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Tenant_Lifecycle_AspNetCore_references_only_tenant_lifecycle_contracts_and_orchestrator()
    {
        var references = ReadProjectReferences("src/Platform.Tenant.Lifecycle.AspNetCore/Platform.Tenant.Lifecycle.AspNetCore.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Tenant.Lifecycle.Contracts", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Tenant.Lifecycle", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Tenant_Lifecycle_does_not_reference_forbidden_packages()
    {
        var packages = ReadPackageReferences("src/Platform.Tenant.Lifecycle/Platform.Tenant.Lifecycle.csproj");
        var violations = packages.Where(package => new[]
        {
            "Microsoft.EntityFrameworkCore",
            "Hangfire",
            "Quartz",
        }.Any(prefix => package.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))).ToArray();
        Assert.Empty(violations);
    }

    [Fact]
    public void Platform_Admin_Contracts_has_no_package_or_project_references()
    {
        Assert.Empty(ReadPackageReferences("src/Platform.Admin.Contracts/Platform.Admin.Contracts.csproj"));
        Assert.Empty(ReadProjectReferences("src/Platform.Admin.Contracts/Platform.Admin.Contracts.csproj"));
    }

    [Fact]
    public void Platform_Admin_Testing_references_only_admin_contracts()
    {
        var references = ReadProjectReferences("src/Platform.Admin.Testing/Platform.Admin.Testing.csproj");
        Assert.Equal(["Platform.Admin.Contracts"], references);
    }

    [Fact]
    public void Platform_Billing_only_references_core_and_billing_contracts()
    {
        var references = ReadProjectReferences("src/Platform.Billing/Platform.Billing.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Billing.Contracts", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Billing_Testing_references_only_public_billing_contracts_and_core()
    {
        var references = ReadProjectReferences("src/Platform.Billing.Testing/Platform.Billing.Testing.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Billing.Contracts", references, StringComparer.OrdinalIgnoreCase);
    }

    [Theory]
    [InlineData("src/Platform.Billing.Stripe/Platform.Billing.Stripe.csproj")]
    [InlineData("src/Platform.Billing.LemonSqueezy/Platform.Billing.LemonSqueezy.csproj")]
    public void Billing_provider_adapters_reference_only_platform_billing(string relativePath)
    {
        Assert.Equal(["Platform.Billing"], ReadProjectReferences(relativePath));
        Assert.DoesNotContain(ReadProjectReferences(relativePath), reference => reference.Contains("Test", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Identity_contract_packages_do_not_reference_provider_sdks()
    {
        var paths = new[]
        {
            "src/Platform.Identity.Contracts/Platform.Identity.Contracts.csproj",
            "src/Platform.Authorization/Platform.Authorization.csproj",
        };
        var forbidden = new[] { "Microsoft.AspNetCore", "Microsoft.EntityFrameworkCore", "Stripe", "Twilio", "OpenIddict" };
        foreach (var path in paths)
            Assert.DoesNotContain(ReadPackageReferences(path), package => forbidden.Any(package.StartsWith));
    }

    [Fact]
    public void Only_Platform_AspNetCore_declares_a_FrameworkReference()
    {
        foreach (var project in ProductionProjects)
        {
            var references = ReadFrameworkReferences(project);
            if (project.EndsWith("Platform.AspNetCore.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Web.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Identity.AspNetCore.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Admin.AspNetCore.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Webhooks.AspNetCore.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Web.Cors.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Web.Resilience.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.FeatureManagement.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Web.OpenApi.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Web.Versioning.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Observability.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Persistence.Multitenancy.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Realtime.AspNetCore.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Quota.AspNetCore.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Privacy.AspNetCore.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Auditing.AspNetCore.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Jobs.Hangfire.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Web.Composition.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Tenant.Lifecycle.AspNetCore.csproj", StringComparison.OrdinalIgnoreCase)
                || project.EndsWith("Platform.Testing.AspNetCore.csproj", StringComparison.OrdinalIgnoreCase))
            {
                Assert.True(
                    references.Length == 1 && references[0].Equals("Microsoft.AspNetCore.App", StringComparison.OrdinalIgnoreCase),
                    $"Platform.AspNetCore must declare only Microsoft.AspNetCore.App but declares: {string.Join(", ", references)}");
            }
            else
            {
                Assert.True(
                    references.Length == 0,
                    $"{project} must not declare FrameworkReferences but declares: {string.Join(", ", references)}");
            }
        }
    }

    [Fact]
    public void Platform_Web_only_references_Platform_AspNetCore()
    {
        var references = ReadProjectReferences("src/Platform.Web/Platform.Web.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.AspNetCore", StringComparison.OrdinalIgnoreCase),
            "Platform.Web must reference only Platform.AspNetCore but references: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Jobs_does_not_reference_forbidden_packages()
    {
        var packages = ReadPackageReferences("src/Platform.Jobs/Platform.Jobs.csproj");
        var violations = packages
            .Where(p => ForbiddenJobsPackagePrefixes.Any(prefix =>
                p.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Jobs must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Jobs_only_references_Platform_Core()
    {
        var references = ReadProjectReferences("src/Platform.Jobs/Platform.Jobs.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Core", StringComparison.OrdinalIgnoreCase),
            "Platform.Jobs must reference only Platform.Core but references: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Jobs_does_not_reference_visual_flow_projects()
    {
        var fullPath = Path.Combine(RepositoryRoot, "src/Platform.Jobs/Platform.Jobs.csproj");
        var document = XDocument.Load(fullPath);

        var projectReferences = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "ProjectReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .Where(value => !string.IsNullOrWhiteSpace(value))
            .ToArray();

        var violations = projectReferences
            .Where(reference => ForbiddenJobsProjectSegments.Any(segment =>
                reference.Contains(segment, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Jobs must not reference VisualFlow projects but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Jobs_Hangfire_references_only_platform_jobs()
    {
        var references = ReadProjectReferences("src/Platform.Jobs.Hangfire/Platform.Jobs.Hangfire.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Jobs", StringComparison.OrdinalIgnoreCase),
            "Platform.Jobs.Hangfire must reference only Platform.Jobs but references: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Jobs_Hangfire_does_not_reference_forbidden_packages()
    {
        var packages = ReadPackageReferences("src/Platform.Jobs.Hangfire/Platform.Jobs.Hangfire.csproj");
        var forbidden = new[]
        {
            "Microsoft.AspNetCore",
            "Microsoft.EntityFrameworkCore",
            "Quartz",
            "StackExchange.Redis",
            "Stripe",
            "Npgsql",
        };
        var violations = packages
            .Where(p => forbidden.Any(prefix => p.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Jobs.Hangfire must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Mailing_does_not_reference_forbidden_packages()
    {
        var packages = ReadPackageReferences("src/Platform.Mailing/Platform.Mailing.csproj");
        var violations = packages
            .Where(p => ForbiddenMailingPackagePrefixes.Any(prefix =>
                p.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Mailing must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Mailing_only_references_Platform_Core()
    {
        var references = ReadProjectReferences("src/Platform.Mailing/Platform.Mailing.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Core", StringComparison.OrdinalIgnoreCase),
            "Platform.Mailing must reference only Platform.Core but references: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Mailing_does_not_reference_visual_flow_projects()
    {
        var fullPath = Path.Combine(RepositoryRoot, "src/Platform.Mailing/Platform.Mailing.csproj");
        var document = XDocument.Load(fullPath);

        var projectReferences = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "ProjectReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .Where(value => !string.IsNullOrWhiteSpace(value))
            .ToArray();

        var violations = projectReferences
            .Where(reference => ForbiddenMailingProjectSegments.Any(segment =>
                reference.Contains(segment, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Mailing must not reference VisualFlow projects but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Mailing_Smtp_references_only_platform_mailing()
    {
        var references = ReadProjectReferences("src/Platform.Mailing.Smtp/Platform.Mailing.Smtp.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Mailing", StringComparison.OrdinalIgnoreCase),
            "Platform.Mailing.Smtp must reference only Platform.Mailing but references: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Mailing_Smtp_does_not_reference_forbidden_packages()
    {
        var packages = ReadPackageReferences("src/Platform.Mailing.Smtp/Platform.Mailing.Smtp.csproj");
        var forbidden = new[]
        {
            "Microsoft.AspNetCore",
            "Microsoft.EntityFrameworkCore",
            "SendGrid",
            "Mailgun",
            "Razor",
            "Liquid",
        };
        var violations = packages
            .Where(p => forbidden.Any(prefix => p.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Mailing.Smtp must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Mailing_SendGrid_references_only_platform_mailing()
    {
        var references = ReadProjectReferences("src/Platform.Mailing.SendGrid/Platform.Mailing.SendGrid.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Mailing", StringComparison.OrdinalIgnoreCase),
            "Platform.Mailing.SendGrid must reference only Platform.Mailing but references: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Mailing_SendGrid_does_not_reference_forbidden_packages()
    {
        var packages = ReadPackageReferences("src/Platform.Mailing.SendGrid/Platform.Mailing.SendGrid.csproj");
        var forbidden = new[]
        {
            "Microsoft.AspNetCore",
            "Microsoft.EntityFrameworkCore",
            "MailKit",
            "Mailgun",
            "Razor",
            "Liquid",
        };
        var violations = packages
            .Where(p => forbidden.Any(prefix => p.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Mailing.SendGrid must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Eventing_does_not_reference_forbidden_packages()
    {
        var packages = ReadPackageReferences("src/Platform.Eventing/Platform.Eventing.csproj");
        var violations = packages
            .Where(p => ForbiddenEventingPackagePrefixes.Any(prefix =>
                p.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Eventing must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Eventing_only_references_Platform_Core()
    {
        var references = ReadProjectReferences("src/Platform.Eventing/Platform.Eventing.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Core", StringComparison.OrdinalIgnoreCase),
            "Platform.Eventing must reference only Platform.Core but references: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Eventing_Contracts_only_references_Platform_Core()
    {
        var path = "src/Platform.Eventing.Contracts/Platform.Eventing.Contracts.csproj";
        var references = ReadProjectReferences(path);

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Core", StringComparison.OrdinalIgnoreCase),
            "Platform.Eventing.Contracts must reference only Platform.Core but references: " + string.Join(", ", references));
        Assert.DoesNotContain(ReadPackageReferences(path), package =>
            package.StartsWith("Microsoft.AspNetCore", StringComparison.OrdinalIgnoreCase)
                || package.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
                || package.StartsWith("RabbitMQ", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Platform_Eventing_RabbitMq_references_only_platform_eventing_contracts()
    {
        var references = ReadProjectReferences("src/Platform.Eventing.RabbitMq/Platform.Eventing.RabbitMq.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Eventing.Contracts", StringComparison.OrdinalIgnoreCase),
            "Platform.Eventing.RabbitMq must reference only Platform.Eventing.Contracts but references: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Eventing_RabbitMq_does_not_reference_forbidden_packages()
    {
        var packages = ReadPackageReferences("src/Platform.Eventing.RabbitMq/Platform.Eventing.RabbitMq.csproj");
        var forbidden = new[]
        {
            "Microsoft.AspNetCore",
            "Microsoft.EntityFrameworkCore",
            "StackExchange.Redis",
            "Stripe",
            "Npgsql",
            "MailKit",
            "SendGrid",
            "Hangfire",
            "Quartz",
            "MassTransit",
        };
        var violations = packages
            .Where(p => forbidden.Any(prefix => p.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Eventing.RabbitMq must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Caching_only_references_Platform_Core()
    {
        var path = "src/Platform.Caching/Platform.Caching.csproj";
        Assert.Equal(["Platform.Core"], ReadProjectReferences(path));
        Assert.DoesNotContain(ReadPackageReferences(path), package =>
            package.StartsWith("Microsoft.AspNetCore", StringComparison.OrdinalIgnoreCase)
                || package.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
                || package.StartsWith("StackExchange.Redis", StringComparison.OrdinalIgnoreCase)
                || package.StartsWith("Microsoft.Extensions.Caching.Hybrid", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Platform_Storage_only_references_Platform_Core()
    {
        var path = "src/Platform.Storage/Platform.Storage.csproj";
        Assert.Equal(["Platform.Core"], ReadProjectReferences(path));
        Assert.DoesNotContain(ReadPackageReferences(path), package =>
            package.StartsWith("AWSSDK", StringComparison.OrdinalIgnoreCase)
                || package.StartsWith("Amazon", StringComparison.OrdinalIgnoreCase)
                || package.StartsWith("Microsoft.AspNetCore", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Platform_Quota_only_references_Platform_Core()
    {
        var path = "src/Platform.Quota/Platform.Quota.csproj";
        Assert.Equal(["Platform.Core"], ReadProjectReferences(path));
        Assert.DoesNotContain(ReadPackageReferences(path), package =>
            package.StartsWith("Microsoft.AspNetCore", StringComparison.OrdinalIgnoreCase)
                || package.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
                || package.StartsWith("StackExchange.Redis", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Platform_Quota_Testing_references_only_quota()
    {
        Assert.Equal(["Platform.Quota"], ReadProjectReferences("src/Platform.Quota.Testing/Platform.Quota.Testing.csproj"));
        Assert.Empty(ReadPackageReferences("src/Platform.Quota.Testing/Platform.Quota.Testing.csproj"));
    }

    [Fact]
    public void Platform_Quota_AspNetCore_references_only_platform_core_and_quota()
    {
        var references = ReadProjectReferences("src/Platform.Quota.AspNetCore/Platform.Quota.AspNetCore.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Quota", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Privacy_only_references_Platform_Core()
    {
        var path = "src/Platform.Privacy/Platform.Privacy.csproj";
        Assert.Equal(["Platform.Core"], ReadProjectReferences(path));
        Assert.DoesNotContain(ReadPackageReferences(path), package =>
            package.StartsWith("Microsoft.AspNetCore", StringComparison.OrdinalIgnoreCase)
                || package.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
                || package.StartsWith("StackExchange.Redis", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Platform_Privacy_AspNetCore_references_only_platform_core_and_privacy()
    {
        var references = ReadProjectReferences("src/Platform.Privacy.AspNetCore/Platform.Privacy.AspNetCore.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Privacy", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Privacy_AspNetCore_does_not_reference_forbidden_packages()
    {
        var path = "src/Platform.Privacy.AspNetCore/Platform.Privacy.AspNetCore.csproj";
        var packages = ReadPackageReferences(path);
        var violations = packages
            .Where(p => p.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("StackExchange.Redis", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Stripe", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Hangfire", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Quartz", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("RabbitMQ", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Polly", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Privacy.AspNetCore must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Quota_AspNetCore_does_not_reference_forbidden_packages()
    {
        var path = "src/Platform.Quota.AspNetCore/Platform.Quota.AspNetCore.csproj";
        var packages = ReadPackageReferences(path);
        var violations = packages
            .Where(p => p.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("StackExchange.Redis", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Stripe", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Hangfire", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Quartz", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("RabbitMQ", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Polly", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Quota.AspNetCore must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Theory]
    [InlineData("src/Platform.Storage.Local/Platform.Storage.Local.csproj")]
    [InlineData("src/Platform.Storage.S3/Platform.Storage.S3.csproj")]
    public void Platform_Storage_adapters_reference_only_base_storage(string path)
    {
        Assert.Equal(["Platform.Storage"], ReadProjectReferences(path));
        Assert.DoesNotContain(ReadProjectReferences(path), reference => reference.Contains("Test", StringComparison.OrdinalIgnoreCase));
    }

    [Theory]
    [InlineData("src/Platform.Caching.Hybrid/Platform.Caching.Hybrid.csproj")]
    [InlineData("src/Platform.Caching.Redis/Platform.Caching.Redis.csproj")]
    public void Platform_Caching_adapters_reference_only_base_caching(string path)
    {
        Assert.Equal(["Platform.Caching"], ReadProjectReferences(path));
        Assert.DoesNotContain(ReadProjectReferences(path), reference => reference.Contains("Test", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Platform_Eventing_EfCore_references_only_core_and_eventing_contracts()
    {
        var references = ReadProjectReferences("src/Platform.Eventing.EfCore/Platform.Eventing.EfCore.csproj");

        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Eventing.Contracts", references, StringComparer.OrdinalIgnoreCase);
        Assert.DoesNotContain(references, reference => reference.Contains("Platform.Eventing.Tests", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Platform_Eventing_does_not_reference_visual_flow_projects()
    {
        var fullPath = Path.Combine(RepositoryRoot, "src/Platform.Eventing/Platform.Eventing.csproj");
        var document = XDocument.Load(fullPath);

        var projectReferences = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "ProjectReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .Where(value => !string.IsNullOrWhiteSpace(value))
            .ToArray();

        var violations = projectReferences
            .Where(reference => ForbiddenEventingProjectSegments.Any(segment =>
                reference.Contains(segment, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Eventing must not reference VisualFlow projects but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Idempotency_does_not_reference_forbidden_packages()
    {
        var packages = ReadPackageReferences("src/Platform.Idempotency/Platform.Idempotency.csproj");
        var violations = packages
            .Where(p => ForbiddenIdempotencyPackagePrefixes.Any(prefix =>
                p.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Idempotency must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Idempotency_only_references_Platform_Core()
    {
        var references = ReadProjectReferences("src/Platform.Idempotency/Platform.Idempotency.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Core", StringComparison.OrdinalIgnoreCase),
            "Platform.Idempotency must reference only Platform.Core but references: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Idempotency_does_not_reference_visual_flow_projects()
    {
        var fullPath = Path.Combine(RepositoryRoot, "src/Platform.Idempotency/Platform.Idempotency.csproj");
        var document = XDocument.Load(fullPath);

        var projectReferences = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "ProjectReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .Where(value => !string.IsNullOrWhiteSpace(value))
            .ToArray();

        var violations = projectReferences
            .Where(reference => ForbiddenIdempotencyProjectSegments.Any(segment =>
                reference.Contains(segment, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Idempotency must not reference VisualFlow projects but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_RateLimiting_does_not_reference_forbidden_packages()
    {
        var packages = ReadPackageReferences("src/Platform.RateLimiting/Platform.RateLimiting.csproj");
        var violations = packages
            .Where(p => ForbiddenRateLimitingPackagePrefixes.Any(prefix =>
                p.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.RateLimiting must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_RateLimiting_only_references_Platform_Core()
    {
        var references = ReadProjectReferences("src/Platform.RateLimiting/Platform.RateLimiting.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Core", StringComparison.OrdinalIgnoreCase),
            "Platform.RateLimiting must reference only Platform.Core but references: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_RateLimiting_does_not_reference_visual_flow_projects()
    {
        var fullPath = Path.Combine(RepositoryRoot, "src/Platform.RateLimiting/Platform.RateLimiting.csproj");
        var document = XDocument.Load(fullPath);

        var projectReferences = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "ProjectReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .Where(value => !string.IsNullOrWhiteSpace(value))
            .ToArray();

        var violations = projectReferences
            .Where(reference => ForbiddenRateLimitingProjectSegments.Any(segment =>
                reference.Contains(segment, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.RateLimiting must not reference VisualFlow projects but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Realtime_does_not_reference_forbidden_packages()
    {
        var packages = ReadPackageReferences("src/Platform.Realtime/Platform.Realtime.csproj");
        var violations = packages
            .Where(p => ForbiddenProductionPackagePrefixes.Any(prefix =>
                p.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Realtime must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Realtime_only_references_Platform_Core()
    {
        var references = ReadProjectReferences("src/Platform.Realtime/Platform.Realtime.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Core", StringComparison.OrdinalIgnoreCase),
            "Platform.Realtime must reference only Platform.Core but references: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Realtime_AspNetCore_does_not_reference_forbidden_packages()
    {
        var path = "src/Platform.Realtime.AspNetCore/Platform.Realtime.AspNetCore.csproj";
        var packages = ReadPackageReferences(path);
        var violations = packages
            .Where(p => p.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("StackExchange.Redis", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Stripe", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Hangfire", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Quartz", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("RabbitMQ", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Realtime.AspNetCore must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Realtime_AspNetCore_references_only_platform_core_and_realtime()
    {
        var references = ReadProjectReferences("src/Platform.Realtime.AspNetCore/Platform.Realtime.AspNetCore.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Realtime", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Realtime_AspNetCore_declares_only_the_aspnetcore_framework_reference()
    {
        var references = ReadFrameworkReferences("src/Platform.Realtime.AspNetCore/Platform.Realtime.AspNetCore.csproj");
        Assert.True(
            references.Length == 1 && references[0].Equals("Microsoft.AspNetCore.App", StringComparison.OrdinalIgnoreCase),
            "Platform.Realtime.AspNetCore must declare only Microsoft.AspNetCore.App but declares: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Webhooks_Contracts_does_not_reference_forbidden_packages()
    {
        var path = "src/Platform.Webhooks.Contracts/Platform.Webhooks.Contracts.csproj";
        var packages = ReadPackageReferences(path);
        var violations = packages
            .Where(p => ForbiddenWebhooksContractsPackagePrefixes.Any(prefix =>
                p.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();
        Assert.True(
            violations.Length == 0,
            "Platform.Webhooks.Contracts must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Webhooks_Contracts_only_references_Platform_Core()
    {
        var path = "src/Platform.Webhooks.Contracts/Platform.Webhooks.Contracts.csproj";
        Assert.Equal(["Platform.Core"], ReadProjectReferences(path));
    }

    [Fact]
    public void Platform_Webhooks_AspNetCore_does_not_reference_visual_flow_projects()
    {
        var fullPath = Path.Combine(RepositoryRoot, "src/Platform.Webhooks.AspNetCore/Platform.Webhooks.AspNetCore.csproj");
        var document = XDocument.Load(fullPath);

        var projectReferences = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "ProjectReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .Where(value => !string.IsNullOrWhiteSpace(value))
            .ToArray();

        var violations = projectReferences
            .Where(reference => ForbiddenWebhooksAspNetCoreProjectSegments.Any(segment =>
                reference.Contains(segment, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Webhooks.AspNetCore must not reference VisualFlow projects but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Auditing_Contracts_only_references_Platform_Core()
    {
        var path = "src/Platform.Auditing.Contracts/Platform.Auditing.Contracts.csproj";
        Assert.Equal(["Platform.Core"], ReadProjectReferences(path));
        Assert.DoesNotContain(ReadPackageReferences(path), package =>
            package.StartsWith("Microsoft.AspNetCore", StringComparison.OrdinalIgnoreCase)
                || package.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Platform_Auditing_Contracts_has_no_package_references_other_than_extensions_abstractions()
    {
        var path = "src/Platform.Auditing.Contracts/Platform.Auditing.Contracts.csproj";
        var packages = ReadPackageReferences(path);
        var violations = packages
            .Where(p => !p.StartsWith("Microsoft.Extensions.", StringComparison.OrdinalIgnoreCase))
            .ToArray();
        Assert.True(
            violations.Length == 0,
            "Platform.Auditing.Contracts must only reference Microsoft.Extensions.* but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Auditing_AspNetCore_references_only_core_and_contracts()
    {
        var references = ReadProjectReferences("src/Platform.Auditing.AspNetCore/Platform.Auditing.AspNetCore.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Auditing.Contracts", references, StringComparer.OrdinalIgnoreCase);

        var packages = ReadPackageReferences("src/Platform.Auditing.AspNetCore/Platform.Auditing.AspNetCore.csproj");
        var violations = packages
            .Where(p => p.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Stripe", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Hangfire", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Quartz", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("RabbitMQ", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Polly", StringComparison.OrdinalIgnoreCase))
            .ToArray();
        Assert.True(
            violations.Length == 0,
            "Platform.Auditing.AspNetCore must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Auditing_AspNetCore_declares_only_the_aspnetcore_framework_reference()
    {
        var references = ReadFrameworkReferences("src/Platform.Auditing.AspNetCore/Platform.Auditing.AspNetCore.csproj");
        Assert.True(
            references.Length == 1 && references[0].Equals("Microsoft.AspNetCore.App", StringComparison.OrdinalIgnoreCase),
            "Platform.Auditing.AspNetCore must declare only Microsoft.AspNetCore.App but declares: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Auditing_EfCore_references_only_core_and_contracts()
    {
        var references = ReadProjectReferences("src/Platform.Auditing.EfCore/Platform.Auditing.EfCore.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Auditing.Contracts", references, StringComparer.OrdinalIgnoreCase);

        var packages = ReadPackageReferences("src/Platform.Auditing.EfCore/Platform.Auditing.EfCore.csproj");
        var violations = packages
            .Where(p => p.StartsWith("Microsoft.AspNetCore", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Stripe", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("StackExchange.Redis", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("RabbitMQ", StringComparison.OrdinalIgnoreCase))
            .ToArray();
        Assert.True(
            violations.Length == 0,
            "Platform.Auditing.EfCore must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Web_Telemetry_does_not_reference_forbidden_packages()
    {
        var path = "src/Platform.Web.Telemetry/Platform.Web.Telemetry.csproj";
        var packages = ReadPackageReferences(path);
        var violations = packages
            .Where(p => p.StartsWith("Microsoft.AspNetCore", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Swashbuckle", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("NSwag", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Polly", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Web.Telemetry must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Web_Telemetry_only_references_Platform_Core()
    {
        var references = ReadProjectReferences("src/Platform.Web.Telemetry/Platform.Web.Telemetry.csproj");

        Assert.True(
            references.Length == 1 && references[0].Equals("Platform.Core", StringComparison.OrdinalIgnoreCase),
            "Platform.Web.Telemetry must reference only Platform.Core but references: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Web_Cors_does_not_reference_forbidden_packages()
    {
        var path = "src/Platform.Web.Cors/Platform.Web.Cors.csproj";
        var packages = ReadPackageReferences(path);
        var violations = packages
            .Where(p => p.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Web.Cors must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Web_Cors_references_only_platform_core_and_telemetry()
    {
        var references = ReadProjectReferences("src/Platform.Web.Cors/Platform.Web.Cors.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Web.Telemetry", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Web_Resilience_does_not_reference_forbidden_packages()
    {
        var path = "src/Platform.Web.Resilience/Platform.Web.Resilience.csproj";
        var packages = ReadPackageReferences(path);
        var violations = packages
            .Where(p => p.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Polly", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Web.Resilience must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Web_Resilience_references_only_platform_core_and_telemetry()
    {
        var references = ReadProjectReferences("src/Platform.Web.Resilience/Platform.Web.Resilience.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Web.Telemetry", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Web_OpenApi_does_not_reference_forbidden_packages()
    {
        var path = "src/Platform.Web.OpenApi/Platform.Web.OpenApi.csproj";
        var packages = ReadPackageReferences(path);
        var violations = packages
            .Where(p => p.StartsWith("Swashbuckle", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("NSwag", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Web.OpenApi must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Web_OpenApi_references_only_platform_core_and_telemetry()
    {
        var references = ReadProjectReferences("src/Platform.Web.OpenApi/Platform.Web.OpenApi.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Web.Telemetry", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Web_Versioning_does_not_reference_forbidden_packages()
    {
        var path = "src/Platform.Web.Versioning/Platform.Web.Versioning.csproj";
        var packages = ReadPackageReferences(path);
        var violations = packages
            .Where(p => p.StartsWith("Swashbuckle", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("NSwag", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Stripe", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Polly", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Hangfire", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Quartz", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("RabbitMQ", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("StackExchange.Redis", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Web.Versioning must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Web_Versioning_references_only_platform_core()
    {
        var references = ReadProjectReferences("src/Platform.Web.Versioning/Platform.Web.Versioning.csproj");
        Assert.Single(references);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Web_Versioning_declares_only_the_aspnetcore_framework_reference()
    {
        var references = ReadFrameworkReferences("src/Platform.Web.Versioning/Platform.Web.Versioning.csproj");
        Assert.True(
            references.Length == 1 && references[0].Equals("Microsoft.AspNetCore.App", StringComparison.OrdinalIgnoreCase),
            "Platform.Web.Versioning must declare only Microsoft.AspNetCore.App but declares: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Observability_does_not_reference_forbidden_packages()
    {
        var path = "src/Platform.Observability/Platform.Observability.csproj";
        var packages = ReadPackageReferences(path);
        var violations = packages
            .Where(p => p.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Npgsql", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("StackExchange.Redis", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("AWSSDK", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("OpenTelemetry", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Serilog", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Stripe", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Observability must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Observability_references_only_platform_core_and_web_telemetry()
    {
        var references = ReadProjectReferences("src/Platform.Observability/Platform.Observability.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Web.Telemetry", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_FeatureManagement_references_only_platform_core_and_web_telemetry()
    {
        var references = ReadProjectReferences("src/Platform.FeatureManagement/Platform.FeatureManagement.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Web.Telemetry", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_FeatureManagement_declares_only_the_aspnetcore_framework_reference()
    {
        var references = ReadFrameworkReferences("src/Platform.FeatureManagement/Platform.FeatureManagement.csproj");
        Assert.True(
            references.Length == 1 && references[0].Equals("Microsoft.AspNetCore.App", StringComparison.OrdinalIgnoreCase),
            "Platform.FeatureManagement must declare only Microsoft.AspNetCore.App but declares: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_FeatureManagement_does_not_reference_forbidden_packages()
    {
        var path = "src/Platform.FeatureManagement/Platform.FeatureManagement.csproj";
        var packages = ReadPackageReferences(path);
        var violations = packages
            .Where(p => p.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Polly", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Stripe", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Hangfire", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Quartz", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("RabbitMQ", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("StackExchange.Redis", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.FeatureManagement must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Http_Resilience_references_only_platform_core_and_web_telemetry()
    {
        var references = ReadProjectReferences("src/Platform.Http.Resilience/Platform.Http.Resilience.csproj");
        Assert.Equal(2, references.Length);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Web.Telemetry", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Http_Resilience_does_not_declare_a_framework_reference()
    {
        var references = ReadFrameworkReferences("src/Platform.Http.Resilience/Platform.Http.Resilience.csproj");
        Assert.True(
            references.Length == 0,
            "Platform.Http.Resilience must not declare FrameworkReferences but declares: " + string.Join(", ", references));
    }

    [Fact]
    public void Platform_Http_Resilience_does_not_reference_forbidden_packages()
    {
        var path = "src/Platform.Http.Resilience/Platform.Http.Resilience.csproj";
        var packages = ReadPackageReferences(path);
        var violations = packages
            .Where(p => p.StartsWith("Microsoft.AspNetCore", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Microsoft.EntityFrameworkCore", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Microsoft.FeatureManagement", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Polly", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Stripe", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("StackExchange.Redis", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.True(
            violations.Length == 0,
            "Platform.Http.Resilience must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Persistence_Multitenancy_does_not_reference_forbidden_packages()
    {
        var path = "src/Platform.Persistence.Multitenancy/Platform.Persistence.Multitenancy.csproj";
        var packages = ReadPackageReferences(path);
        var violations = packages
            .Where(p => p.StartsWith("Microsoft.AspNetCore", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Npgsql", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Finbuckle", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("StackExchange.Redis", StringComparison.OrdinalIgnoreCase)
                || p.StartsWith("Stripe", StringComparison.OrdinalIgnoreCase))
            .ToArray();
        Assert.True(
            violations.Length == 0,
            "Platform.Persistence.Multitenancy must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Persistence_Multitenancy_references_only_platform_core_aspnetcore_and_persistence()
    {
        var references = ReadProjectReferences("src/Platform.Persistence.Multitenancy/Platform.Persistence.Multitenancy.csproj");
        Assert.Equal(3, references.Length);
        Assert.Contains("Platform.Core", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.AspNetCore", references, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.Persistence.EfCore", references, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Platform_Core_tenant_contracts_have_no_package_dependencies()
    {
        var path = "src/Platform.Core/Platform.Core.csproj";
        var packages = ReadPackageReferences(path);
        Assert.Empty(packages);
    }

    [Fact]
    public void Platform_Domain_only_references_Platform_Core()
    {
        Assert.Equal(["Platform.Core"], ReadProjectReferences("src/Platform.Domain/Platform.Domain.csproj"));
    }

    [Fact]
    public void Platform_Domain_does_not_reference_forbidden_packages()
    {
        var packages = ReadPackageReferences("src/Platform.Domain/Platform.Domain.csproj");
        var violations = packages
            .Where(package => new[]
            {
                "Microsoft.AspNetCore",
                "Microsoft.EntityFrameworkCore",
                "Mediator",
                "MediatR",
                "FluentValidation",
                "Hangfire",
                "Quartz",
                "RabbitMQ",
                "MassTransit",
                "StackExchange.Redis",
                "Stripe",
                "MailKit",
                "SendGrid",
                "Npgsql",
                "MongoDB",
            }.Any(prefix => package.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();
        Assert.True(
            violations.Length == 0,
            "Platform.Domain must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Domain_does_not_declare_a_framework_reference()
    {
        Assert.Empty(ReadFrameworkReferences("src/Platform.Domain/Platform.Domain.csproj"));
    }

    [Fact]
    public void Platform_Web_Composition_only_references_Platform_Core()
    {
        Assert.Equal(["Platform.Core"], ReadProjectReferences("src/Platform.Web.Composition/Platform.Web.Composition.csproj"));
    }

    [Fact]
    public void Platform_Web_Composition_does_not_reference_forbidden_packages()
    {
        var packages = ReadPackageReferences("src/Platform.Web.Composition/Platform.Web.Composition.csproj");
        var violations = packages
            .Where(package => ForbiddenWebCompositionPackagePrefixes.Any(prefix =>
                package.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();
        Assert.True(
            violations.Length == 0,
            "Platform.Web.Composition must not reference forbidden packages but references: " + string.Join(", ", violations));
    }

    [Fact]
    public void Platform_Web_Composition_declares_only_the_aspnetcore_framework_reference()
    {
        var references = ReadFrameworkReferences("src/Platform.Web.Composition/Platform.Web.Composition.csproj");
        Assert.True(
            references.Length == 1 && references[0].Equals("Microsoft.AspNetCore.App", StringComparison.OrdinalIgnoreCase),
            "Platform.Web.Composition must declare only Microsoft.AspNetCore.App but declares: " + string.Join(", ", references));
    }

    [Fact]
    public void Consumer_conformance_project_does_not_reference_platform_projects()
    {
        var path = "tests/Platform.ConsumerConformance/Platform.ConsumerConformance.csproj";
        var fullPath = Path.Combine(RepositoryRoot, path);
        Assert.True(File.Exists(fullPath), "Consumer conformance project must exist at " + path);

        var document = XDocument.Load(fullPath);
        var projectReferences = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "ProjectReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .Where(value => !string.IsNullOrWhiteSpace(value))
            .ToArray();

        Assert.True(
            projectReferences.Length == 0,
            "Consumer conformance project must only consume platform packages via <PackageReference>; found: " + string.Join(", ", projectReferences));
    }

    [Fact]
    public void Consumer_conformance_project_declares_a_local_nuget_feed()
    {
        var configPath = Path.Combine(RepositoryRoot, "tests/Platform.ConsumerConformance/nuget.config");
        Assert.True(File.Exists(configPath), "Consumer conformance project must ship a nuget.config");

        var document = XDocument.Load(configPath);
        var sources = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "add", StringComparison.OrdinalIgnoreCase))
            .Select(e => new { Key = e.Attribute("key")?.Value, Value = e.Attribute("value")?.Value })
            .Where(e => !string.IsNullOrWhiteSpace(e.Key))
            .ToArray();

        var localFeed = Assert.Single(sources, s => string.Equals(s.Key, "local-platform-feed", StringComparison.OrdinalIgnoreCase));
        Assert.Equal(".local-feed", localFeed.Value);
    }

    [Fact]
    public void Consumer_conformance_build_script_records_environment_blockers()
    {
        var scriptPath = Path.Combine(RepositoryRoot, "scripts/conformance.sh");
        Assert.True(File.Exists(scriptPath), "Conformance build script must exist at scripts/conformance.sh");
        var contents = File.ReadAllText(scriptPath);
        Assert.Contains("ENV BLOCKER", contents, StringComparison.Ordinal);
        Assert.Contains("logs", contents, StringComparison.Ordinal);
    }

    [Fact]
    public void Production_project_does_not_reference_consumer_conformance_project()
    {
        foreach (var project in ProductionProjects)
        {
            var references = ReadProjectReferences(project);
            var leaks = references
                .Where(r => r.Contains("ConsumerConformance", StringComparison.OrdinalIgnoreCase))
                .ToArray();
            Assert.True(
                leaks.Length == 0,
                $"{project} must not reference the consumer conformance project but references: {string.Join(", ", leaks)}");
        }
    }

    [Fact]
    public void Package_manifest_exists_and_is_generated_alongside_its_script()
    {
        var manifestPath = Path.Combine(RepositoryRoot, "eng/package-manifest.json");
        var scriptPath = Path.Combine(RepositoryRoot, "scripts/generate-package-manifest.sh");
        Assert.True(File.Exists(manifestPath), "Package manifest must exist at " + manifestPath);
        Assert.True(File.Exists(scriptPath), "Manifest generator script must exist at " + scriptPath);
        var contents = File.ReadAllText(scriptPath);
        Assert.Contains("--check", contents, StringComparison.Ordinal);
    }

    [Fact]
    public void Package_manifest_is_in_sync_with_source()
    {
        var scriptPath = Path.Combine(RepositoryRoot, "scripts/generate-package-manifest.sh");
        if (!File.Exists(scriptPath))
        {
            return;
        }
        var exitCode = RunCommand(scriptPath, "--check");
        Assert.True(
            exitCode == 0,
            "scripts/generate-package-manifest.sh --check must succeed; regenerate the manifest if it drifted.");
    }

    [Fact]
    public void Consumer_conformance_project_pins_every_Platform_package()
    {
        var path = Path.Combine(RepositoryRoot, "tests/Platform.ConsumerConformance/Platform.ConsumerConformance.csproj");
        Assert.True(File.Exists(path), "Consumer conformance project must exist.");
        var document = XDocument.Load(path);
        foreach (var reference in document
                     .Descendants()
                     .Where(e => string.Equals(e.Name.LocalName, "PackageReference", StringComparison.OrdinalIgnoreCase))
                     .Where(e => (e.Attribute("Include")?.Value ?? string.Empty).StartsWith("Platform.", StringComparison.OrdinalIgnoreCase)))
        {
            var name = reference.Attribute("Include")?.Value ?? string.Empty;
            var version = reference.Attribute("Version")?.Value ?? string.Empty;
            Assert.False(string.IsNullOrWhiteSpace(version), $"{name} must declare a pinned Version attribute.");
            Assert.False(version.Contains('*') || version.Contains('[') || version.Contains('('), $"{name} must use an exact version. Saw '{version}'.");
        }
    }

    [Fact]
    public void Consumer_conformance_project_does_not_reference_testing_packages_from_production_projects()
    {
        // Production projects must never appear as a PackageReference target of the
        // consumer conformance fixture — only Platform.* packages are allowed.
        var path = Path.Combine(RepositoryRoot, "tests/Platform.ConsumerConformance/Platform.ConsumerConformance.csproj");
        var document = XDocument.Load(path);
        var references = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "ProjectReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .Where(value => !string.IsNullOrWhiteSpace(value))
            .ToArray();
        Assert.Empty(references);
    }

    private static int RunCommand(string script, string arg)
    {
        var psi = new System.Diagnostics.ProcessStartInfo("bash", $"\"{script}\" {arg}")
        {
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            WorkingDirectory = RepositoryRoot,
        };
        using var process = System.Diagnostics.Process.Start(psi) ?? throw new InvalidOperationException("Failed to start " + script);
        process.WaitForExit();
        return process.ExitCode;
    }

    private static string LocateRepositoryRoot()
    {
        var current = new DirectoryInfo(AppContext.BaseDirectory);
        while (current is not null)
        {
            if (File.Exists(Path.Combine(current.FullName, "Directory.Build.props"))
                && File.Exists(Path.Combine(current.FullName, "Platform.sln")))
            {
                return current.FullName;
            }
            current = current.Parent;
        }

        throw new InvalidOperationException(
            "Unable to locate the repository root from " + AppContext.BaseDirectory);
    }

    private static string[] ReadProjectReferences(string relativePath)
    {
        var fullPath = Path.Combine(RepositoryRoot, relativePath);
        var document = XDocument.Load(fullPath);

        return document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "ProjectReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .Where(value => !string.IsNullOrWhiteSpace(value))
            .Select(value => Path.GetFileNameWithoutExtension(value.Split('\\', '/').Last()) ?? string.Empty)
            .Where(name => !string.IsNullOrWhiteSpace(name))
            .ToArray();
    }

    private static string[] ReadPackageReferences(string relativePath)
    {
        var fullPath = Path.Combine(RepositoryRoot, relativePath);
        var document = XDocument.Load(fullPath);

        return document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "PackageReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .Where(value => !string.IsNullOrWhiteSpace(value))
            .ToArray();
    }

    private static string[] ReadFrameworkReferences(string relativePath)
    {
        var fullPath = Path.Combine(RepositoryRoot, relativePath);
        var document = XDocument.Load(fullPath);

        return document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "FrameworkReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .Where(value => !string.IsNullOrWhiteSpace(value))
            .ToArray();
    }
}
