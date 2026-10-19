using System.Security.Cryptography;
using System.Text.Json;
using Platform.Adoption;

namespace Platform.Adoption.Tests;

public sealed class AdoptionAnalyzerTests
{
    [Theory]
    [InlineData("")]
    [InlineData("relative/path")]
    [InlineData("Fixtures/minimal")]
    public void Rejects_unusable_targets(string target)
    {
        Assert.Throws<AdoptionTargetException>(() => AdoptionAnalyzer.Analyze(target));
    }

    [Fact]
    public void Rejects_missing_absolute_directory()
    {
        var missing = Path.Combine(Fixture("minimal"), "does-not-exist");
        Assert.Throws<AdoptionTargetException>(() => AdoptionAnalyzer.Analyze(missing));
    }

    [Fact]
    public void Minimal_fixture_reports_warnings_without_failures()
    {
        var report = AdoptionAnalyzer.Analyze(Fixture("minimal"));

        Assert.Equal(AdoptionExitCodes.Success, report.ExitCode);
        Assert.False(report.HasFailures);
        Assert.Equal(report.TargetDirectory, Fixture("minimal"));
        AssertStatus(report, "sdk", AdoptionStatus.Warning);
        AssertStatus(report, "solution", AdoptionStatus.Warning);
        AssertStatus(report, "projects", AdoptionStatus.Pass);
        AssertStatus(report, "central-packages", AdoptionStatus.Warning);
        AssertStatus(report, "platform-pinning", AdoptionStatus.Pass);
        AssertStatus(report, "test-boundary", AdoptionStatus.Pass);
        AssertStatus(report, "nullable-warnings", AdoptionStatus.Warning);
    }

    [Fact]
    public void Adopted_fixture_is_clean_with_no_proposed_edits()
    {
        var report = AdoptionAnalyzer.Analyze(Fixture("adopted"));

        Assert.Equal(AdoptionExitCodes.Success, report.ExitCode);
        Assert.All(report.Results, result => Assert.Equal(AdoptionStatus.Pass, result.Status));
        Assert.Empty(report.ProposedEdits);
    }

    [Fact]
    public void Misconfigured_fixture_fails_pinning_and_boundary_with_preview()
    {
        var before = Snapshot(Fixture("misconfigured"));
        var report = AdoptionAnalyzer.Analyze(Fixture("misconfigured"));

        Assert.Equal(AdoptionExitCodes.Failures, report.ExitCode);
        Assert.True(report.HasFailures);
        var pinning = AssertStatus(report, "platform-pinning", AdoptionStatus.Failed);
        Assert.Contains("0.1.*", pinning.Evidence, StringComparison.Ordinal);
        var boundary = AssertStatus(report, "test-boundary", AdoptionStatus.Failed);
        Assert.Contains("Broken.csproj", boundary.Evidence, StringComparison.Ordinal);
        Assert.Contains("Platform.Core.Testing", boundary.Evidence, StringComparison.Ordinal);

        var floating = Assert.Single(report.ProposedEdits, edit => edit.PackageId == "Platform.Core");
        Assert.Equal("0.1.*", floating.CurrentVersion);
        Assert.Equal("0.1.0", floating.ProposedVersion);
        Assert.Equal(before, Snapshot(Fixture("misconfigured")));
    }

    [Fact]
    public void Targets_explicit_sibling_directory_not_working_directory()
    {
        var report = AdoptionAnalyzer.Analyze(Fixture("adopted"));

        Assert.Equal(Fixture("adopted"), report.TargetDirectory);
        Assert.NotEqual(
            Path.GetFullPath(Directory.GetCurrentDirectory()),
            report.TargetDirectory);
        var projects = AssertStatus(report, "projects", AdoptionStatus.Pass);
        Assert.Contains("Adopted.csproj", projects.Evidence, StringComparison.Ordinal);
    }

    [Fact]
    public void Unreachable_feed_is_environment_blocked_with_rerun_instruction()
    {
        var options = new AdoptionOptions
        {
            IncludeEnvironmentChecks = true,
            FeedUrl = "http://127.0.0.1:9/index.json",
            EnvironmentProbeTimeout = TimeSpan.FromSeconds(5),
        };
        var report = AdoptionAnalyzer.Analyze(Fixture("adopted"), options);

        Assert.False(report.HasFailures);
        Assert.Equal(1, report.BlockedCount);
        Assert.Equal(AdoptionExitCodes.BlockedOnly, report.ExitCode);
        var feed = AssertStatus(report, "environment-feed", AdoptionStatus.EnvironmentBlocked);
        Assert.Contains("--feed-url", feed.Remediation, StringComparison.Ordinal);
    }

    [Fact]
    public void Environment_checks_are_omitted_by_default()
    {
        var report = AdoptionAnalyzer.Analyze(Fixture("adopted"));

        Assert.DoesNotContain(report.Results, result => result.CheckId.StartsWith("environment-", StringComparison.Ordinal));
    }

    [Fact]
    public void Report_json_is_stable_and_secret_free()
    {
        var report = AdoptionAnalyzer.Analyze(Fixture("misconfigured"));
        var json = AdoptionAnalyzer.ToJson(report);

        using var document = JsonDocument.Parse(json);
        var root = document.RootElement;
        Assert.Equal(Fixture("misconfigured"), root.GetProperty("targetDirectory").GetString());
        Assert.Equal(AdoptionAnalyzer.ToolVersion, root.GetProperty("toolVersion").GetString());
        var results = root.GetProperty("results").EnumerateArray().ToList();
        Assert.Equal(report.Results.Count, results.Count);
        foreach (var result in results)
        {
            Assert.True(result.TryGetProperty("checkId", out _));
            Assert.True(result.TryGetProperty("status", out _));
            Assert.True(result.TryGetProperty("evidence", out _));
            Assert.True(result.TryGetProperty("remediation", out _));
        }

        Assert.DoesNotContain("   at ", json, StringComparison.Ordinal);
        Assert.DoesNotContain("password", json, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("secret", json, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Inventory_lists_projects_and_packages()
    {
        var projects = AdoptionInventory.ListProjects(Fixture("adopted"));

        var adopted = Assert.Single(projects);
        Assert.EndsWith("Adopted.csproj", adopted.RelativePath, StringComparison.Ordinal);
        var platform = Assert.Single(adopted.Packages, package => package.Id == "Platform.Core");
        Assert.Equal("0.1.0", platform.Version);

        using var document = JsonDocument.Parse(AdoptionInventory.ToJson(Fixture("adopted"), projects));
        Assert.Equal(Fixture("adopted"), document.RootElement.GetProperty("target").GetString());
    }

    private static AdoptionCheckResult AssertStatus(AdoptionReport report, string checkId, AdoptionStatus expected)
    {
        var result = Assert.Single(report.Results, r => r.CheckId == checkId);
        Assert.Equal(expected, result.Status);
        return result;
    }

    private static string Fixture(string name)
    {
        var current = new DirectoryInfo(AppContext.BaseDirectory);
        while (current is not null)
        {
            var candidate = Path.Combine(current.FullName, "tests", "Platform.Adoption.Tests", "Fixtures", name);
            if (Directory.Exists(candidate))
            {
                return candidate;
            }

            current = current.Parent;
        }

        throw new Xunit.Sdk.XunitException("Could not locate adoption fixture " + name + ".");
    }

    private static string Snapshot(string directory)
    {
        using var hash = SHA256.Create();
        var combined = string.Empty;
        foreach (var file in Directory.EnumerateFiles(directory, "*", SearchOption.AllDirectories)
                     .OrderBy(path => path, StringComparer.Ordinal))
        {
            var bytes = File.ReadAllBytes(file);
            combined += file + Convert.ToHexString(hash.ComputeHash(bytes));
        }

        return combined;
    }
}
