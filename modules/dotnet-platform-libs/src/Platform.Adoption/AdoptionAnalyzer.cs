using System.Diagnostics;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Xml.Linq;

namespace Platform.Adoption;

/// <summary>
/// Stable process exit codes for adoption tooling automation.
/// </summary>
public static class AdoptionExitCodes
{
    /// <summary>No source failures and no environment-blocked checks.</summary>
    public const int Success = 0;

    /// <summary>At least one source check failed.</summary>
    public const int Failures = 1;

    /// <summary>No source failures, but at least one check was environment-blocked.</summary>
    public const int BlockedOnly = 2;

    /// <summary>Usage error: missing or unusable target directory or arguments.</summary>
    public const int Usage = 64;
}

/// <summary>
/// Deterministic adoption diagnostics over an explicit target directory.
/// Read-only: analysis never creates, modifies, or deletes files.
/// </summary>
public static class AdoptionAnalyzer
{
    /// <summary>Version of the adoption diagnostic contracts.</summary>
    public const string ToolVersion = "0.1.0";

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = true,
        Converters = { new JsonStringEnumConverter(JsonNamingPolicy.CamelCase) },
    };

    /// <summary>
    /// Analyzes the explicit target directory and returns a stable report.
    /// </summary>
    /// <param name="projectDir">Absolute path to the project/repository directory.</param>
    /// <param name="options">Analysis options; defaults apply when null.</param>
    /// <exception cref="AdoptionTargetException">The target is not an absolute existing directory.</exception>
    public static AdoptionReport Analyze(string projectDir, AdoptionOptions? options = null)
    {
        var target = ResolveTarget(projectDir);
        var effective = options ?? new AdoptionOptions();
        var results = new List<AdoptionCheckResult>();
        var edits = new List<ProposedEdit>();

        results.Add(CheckSdk(target));
        results.Add(CheckSolution(target));
        var projects = EnumerateProjects(target);
        results.Add(CheckProjects(projects, target));
        results.Add(CheckCentralPackages(target));
        results.Add(CheckPlatformPinning(target, projects, effective, edits));
        results.Add(CheckTestBoundary(target, projects));
        results.Add(CheckNullableWarnings(target));
        if (effective.IncludeEnvironmentChecks)
        {
            results.AddRange(CheckEnvironment(effective));
        }

        return new AdoptionReport(target, ToolVersion, results, edits);
    }

    /// <summary>
    /// Returns preview-only package alignment suggestions without writing any file.
    /// </summary>
    /// <param name="projectDir">Absolute path to the project/repository directory.</param>
    /// <param name="options">Analysis options; defaults apply when null.</param>
    /// <exception cref="AdoptionTargetException">The target is not an absolute existing directory.</exception>
    public static IReadOnlyList<ProposedEdit> PreviewAlignment(string projectDir, AdoptionOptions? options = null)
    {
        return Analyze(projectDir, options).ProposedEdits;
    }

    /// <summary>Serializes a report to stable machine-readable JSON.</summary>
    /// <param name="report">The report to serialize.</param>
    public static string ToJson(AdoptionReport report)
    {
        ArgumentNullException.ThrowIfNull(report);
        return JsonSerializer.Serialize(report, JsonOptions);
    }

    internal static string ResolveTarget(string projectDir)
    {
        if (string.IsNullOrWhiteSpace(projectDir))
        {
            throw new AdoptionTargetException(projectDir, "A --project-dir value is required.");
        }

        if (!Path.IsPathFullyQualified(projectDir))
        {
            throw new AdoptionTargetException(projectDir, "The --project-dir value must be an absolute path.");
        }

        var full = Path.GetFullPath(projectDir);
        if (!Directory.Exists(full))
        {
            throw new AdoptionTargetException(projectDir, "The --project-dir value must be an existing directory.");
        }

        return full;
    }

    private static List<string> EnumerateProjects(string target)
    {
        return Directory.EnumerateFiles(target, "*.csproj", SearchOption.AllDirectories)
            .Where(path => !IsUnderOutputOrVersionControl(target, path))
            .OrderBy(path => path, StringComparer.Ordinal)
            .ToList();
    }

    private static bool IsUnderOutputOrVersionControl(string target, string path)
    {
        var relative = Path.GetRelativePath(target, path);
        var segments = relative.Split(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        return segments.Any(segment =>
            segment.Equals("bin", StringComparison.OrdinalIgnoreCase)
            || segment.Equals("obj", StringComparison.OrdinalIgnoreCase)
            || segment.Equals(".git", StringComparison.OrdinalIgnoreCase));
    }

    private static string Relative(string target, string path)
    {
        return Path.GetRelativePath(target, path);
    }

    private static XDocument? TryLoadXml(string path)
    {
        try
        {
            return XDocument.Load(path);
        }
        catch (Exception ex) when (ex is IOException || ex is UnauthorizedAccessException || ex is System.Xml.XmlException)
        {
            return null;
        }
    }

    private static AdoptionCheckResult CheckSdk(string target)
    {
        var pin = Path.Combine(target, "global.json");
        if (!File.Exists(pin))
        {
            return new AdoptionCheckResult(
                "sdk",
                "SDK pin",
                AdoptionStatus.Warning,
                "No global.json at the target root.",
                "Add a global.json pinning a .NET 10 SDK so consumer builds match the platform baseline.");
        }

        try
        {
            using var document = JsonDocument.Parse(File.ReadAllText(pin));
            if (document.RootElement.TryGetProperty("sdk", out var sdk)
                && sdk.TryGetProperty("version", out var version)
                && version.GetString() is { Length: > 0 } pinned)
            {
                return new AdoptionCheckResult(
                    "sdk",
                    "SDK pin",
                    AdoptionStatus.Pass,
                    $"global.json pins SDK {pinned}.",
                    string.Empty);
            }
        }
        catch (Exception ex) when (ex is IOException || ex is UnauthorizedAccessException || ex is JsonException)
        {
            return new AdoptionCheckResult(
                "sdk",
                "SDK pin",
                AdoptionStatus.Warning,
                $"global.json exists but was not readable as JSON ({ex.GetType().Name}).",
                "Repair global.json so it declares an sdk.version string.");
        }

        return new AdoptionCheckResult(
            "sdk",
            "SDK pin",
            AdoptionStatus.Warning,
            "global.json exists but declares no sdk.version.",
            "Set sdk.version in global.json to pin the consumer SDK.");
    }

    private static AdoptionCheckResult CheckSolution(string target)
    {
        var solutions = Directory.EnumerateFiles(target, "*.sln", SearchOption.AllDirectories)
            .Where(path => !IsUnderOutputOrVersionControl(target, path))
            .Select(path => Relative(target, path))
            .OrderBy(name => name, StringComparer.Ordinal)
            .ToList();
        return solutions.Count == 0
            ? new AdoptionCheckResult(
                "solution",
                "Solution discoverability",
                AdoptionStatus.Warning,
                "No .sln file discovered under the target.",
                "Add a solution including the consumer projects, or scope --project-dir to a repository that has one.")
            : new AdoptionCheckResult(
                "solution",
                "Solution discoverability",
                AdoptionStatus.Pass,
                $"Discovered {solutions.Count} solution(s): {string.Join(", ", solutions)}.",
                string.Empty);
    }

    private static AdoptionCheckResult CheckProjects(List<string> projects, string target)
    {
        return projects.Count == 0
            ? new AdoptionCheckResult(
                "projects",
                "Project discoverability",
                AdoptionStatus.Failed,
                "No .csproj files discovered under the target.",
                "Point --project-dir at a directory containing C# projects.")
            : new AdoptionCheckResult(
                "projects",
                "Project discoverability",
                AdoptionStatus.Pass,
                $"Discovered {projects.Count} project(s); first: {Relative(target, projects[0])}.",
                string.Empty);
    }

    private static AdoptionCheckResult CheckCentralPackages(string target)
    {
        var central = Path.Combine(target, "Directory.Packages.props");
        if (!File.Exists(central))
        {
            return new AdoptionCheckResult(
                "central-packages",
                "Central package management",
                AdoptionStatus.Warning,
                "No Directory.Packages.props at the target root.",
                "Adopt central package management so Platform.* versions stay exact and reviewable.");
        }

        var text = File.ReadAllText(central);
        return text.Contains("ManagePackageVersionsCentrally", StringComparison.Ordinal)
            && text.Contains("true", StringComparison.Ordinal)
            ? new AdoptionCheckResult(
                "central-packages",
                "Central package management",
                AdoptionStatus.Pass,
                "Directory.Packages.props enables central version management.",
                string.Empty)
            : new AdoptionCheckResult(
                "central-packages",
                "Central package management",
                AdoptionStatus.Warning,
                "Directory.Packages.props exists but does not enable ManagePackageVersionsCentrally.",
                "Set ManagePackageVersionsCentrally to true in Directory.Packages.props.");
    }

    private static AdoptionCheckResult CheckPlatformPinning(
        string target,
        IReadOnlyList<string> projects,
        AdoptionOptions options,
        List<ProposedEdit> edits)
    {
        var failures = new List<string>();
        var warnings = new List<string>();
        var scanned = 0;

        var centralPins = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        foreach (var central in Directory.EnumerateFiles(target, "Directory.Packages.props", SearchOption.AllDirectories)
                     .Where(path => !IsUnderOutputOrVersionControl(target, path)))
        {
            var document = TryLoadXml(central);
            if (document is null)
            {
                continue;
            }

            foreach (var pin in document.Descendants().Where(e => e.Name.LocalName == "PackageVersion"))
            {
                var id = pin.Attribute("Include")?.Value ?? string.Empty;
                var version = pin.Attribute("Version")?.Value ?? string.Empty;
                if (id.StartsWith("Platform.", StringComparison.Ordinal) && version.Length > 0)
                {
                    centralPins[id] = version;
                }
            }
        }

        foreach (var project in projects)
        {
            var document = TryLoadXml(project);
            if (document is null)
            {
                continue;
            }

            foreach (var reference in document.Descendants().Where(e => e.Name.LocalName == "PackageReference"))
            {
                var id = reference.Attribute("Include")?.Value ?? string.Empty;
                if (!id.StartsWith("Platform.", StringComparison.Ordinal))
                {
                    continue;
                }

                scanned++;
                var relative = Relative(target, project);
                var declared = reference.Attribute("Version")?.Value ?? string.Empty;
                var effectiveVersion = declared.Length > 0
                    ? declared
                    : centralPins.TryGetValue(id, out var pinned) ? pinned : string.Empty;
                if (effectiveVersion.Length == 0)
                {
                    warnings.Add($"{relative}: {id} is unpinned");
                    edits.Add(new ProposedEdit(relative, id, "(unpinned)", options.ExpectedPlatformVersion, "Pin an exact Platform.* version."));
                }
                else if (IsFloating(effectiveVersion))
                {
                    failures.Add($"{relative}: {id} uses floating version '{effectiveVersion}'");
                    edits.Add(new ProposedEdit(relative, id, effectiveVersion, options.ExpectedPlatformVersion, "Replace the floating version with an exact pin."));
                }
                else if (!string.Equals(effectiveVersion, options.ExpectedPlatformVersion, StringComparison.Ordinal))
                {
                    warnings.Add($"{relative}: {id} is {effectiveVersion}, expected {options.ExpectedPlatformVersion}");
                    edits.Add(new ProposedEdit(relative, id, effectiveVersion, options.ExpectedPlatformVersion, "Align to the expected platform version."));
                }
            }
        }

        if (scanned == 0)
        {
            return new AdoptionCheckResult(
                "platform-pinning",
                "Platform package pinning",
                AdoptionStatus.Pass,
                "No Platform.* package references discovered.",
                string.Empty);
        }

        if (failures.Count > 0)
        {
            return new AdoptionCheckResult(
                "platform-pinning",
                "Platform package pinning",
                AdoptionStatus.Failed,
                $"Scanned {scanned} Platform.* reference(s). Floating versions: {string.Join("; ", failures)}.",
                "Pin exact versions for every Platform.* reference; use preview to list the proposed edits.");
        }

        return warnings.Count == 0
            ? new AdoptionCheckResult(
                "platform-pinning",
                "Platform package pinning",
                AdoptionStatus.Pass,
                $"All {scanned} Platform.* reference(s) use exact expected pins.",
                string.Empty)
            : new AdoptionCheckResult(
                "platform-pinning",
                "Platform package pinning",
                AdoptionStatus.Warning,
                $"Scanned {scanned} Platform.* reference(s). Alignment notes: {string.Join("; ", warnings)}.",
                "Review the preview output and align pins to the expected platform version.");
    }

    private static bool IsFloating(string version)
    {
        return version.Contains('*', StringComparison.Ordinal)
            || version.Contains('[', StringComparison.Ordinal)
            || version.Contains('(', StringComparison.Ordinal);
    }

    private static AdoptionCheckResult CheckTestBoundary(string target, IReadOnlyList<string> projects)
    {
        var violations = new List<string>();
        var production = 0;
        foreach (var project in projects)
        {
            var relative = Relative(target, project);
            var segments = relative.Split(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
            var fileName = Path.GetFileName(project);
            if (segments.Any(segment => segment.Equals("tests", StringComparison.OrdinalIgnoreCase) || segment.Equals("test", StringComparison.OrdinalIgnoreCase))
                || fileName.EndsWith(".Tests.csproj", StringComparison.OrdinalIgnoreCase)
                || fileName.EndsWith(".Test.csproj", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            production++;
            var document = TryLoadXml(project);
            if (document is null)
            {
                continue;
            }

            foreach (var reference in document.Descendants().Where(e => e.Name.LocalName == "PackageReference"))
            {
                var id = reference.Attribute("Include")?.Value ?? string.Empty;
                if (id.EndsWith(".Testing", StringComparison.OrdinalIgnoreCase))
                {
                    violations.Add($"{relative} references {id}");
                }
            }
        }

        return violations.Count == 0
            ? new AdoptionCheckResult(
                "test-boundary",
                "Test-only package boundary",
                AdoptionStatus.Pass,
                $"Scanned {production} production project(s); no test-only package references.",
                string.Empty)
            : new AdoptionCheckResult(
                "test-boundary",
                "Test-only package boundary",
                AdoptionStatus.Failed,
                $"Production projects reference test-only packages: {string.Join("; ", violations)}.",
                "Move test-only references to test projects; production packages must not depend on test-only packages.");
    }

    private static AdoptionCheckResult CheckNullableWarnings(string target)
    {
        var props = Path.Combine(target, "Directory.Build.props");
        if (!File.Exists(props))
        {
            return new AdoptionCheckResult(
                "nullable-warnings",
                "Nullable and warnings-as-errors signals",
                AdoptionStatus.Warning,
                "No Directory.Build.props at the target root.",
                "Enable Nullable and TreatWarningsAsErrors for the consumer build.");
        }

        var text = File.ReadAllText(props);
        var nullable = text.Contains("<Nullable>enable</Nullable>", StringComparison.Ordinal);
        var warnings = text.Contains("<TreatWarningsAsErrors>true</TreatWarningsAsErrors>", StringComparison.Ordinal);
        return nullable && warnings
            ? new AdoptionCheckResult(
                "nullable-warnings",
                "Nullable and warnings-as-errors signals",
                AdoptionStatus.Pass,
                "Directory.Build.props enables Nullable and TreatWarningsAsErrors.",
                string.Empty)
            : new AdoptionCheckResult(
                "nullable-warnings",
                "Nullable and warnings-as-errors signals",
                AdoptionStatus.Warning,
                $"Directory.Build.props signals: Nullable={(nullable ? "on" : "off")}, WarningsAsErrors={(warnings ? "on" : "off")}.",
                "Enable the missing signals in Directory.Build.props.");
    }

    private static IEnumerable<AdoptionCheckResult> CheckEnvironment(AdoptionOptions options)
    {
        if (!string.IsNullOrWhiteSpace(options.FeedUrl))
        {
            yield return CheckFeed(options.FeedUrl, options.EnvironmentProbeTimeout);
        }

        if (options.CheckDocker)
        {
            yield return CheckDocker(options.EnvironmentProbeTimeout);
        }
    }

    private static AdoptionCheckResult CheckFeed(string feedUrl, TimeSpan timeout)
    {
        try
        {
            using var client = new HttpClient { Timeout = timeout };
            using var response = client.GetAsync(feedUrl).GetAwaiter().GetResult();
            return response.IsSuccessStatusCode
                ? new AdoptionCheckResult(
                    "environment-feed",
                    "Package feed reachability",
                    AdoptionStatus.Pass,
                    "Feed responded successfully.",
                    string.Empty)
                : new AdoptionCheckResult(
                    "environment-feed",
                    "Package feed reachability",
                    AdoptionStatus.EnvironmentBlocked,
                    $"Feed responded with status {(int)response.StatusCode}.",
                    $"Rerun with --feed-url {feedUrl} after the feed is reachable; source checks are unaffected.");
        }
        catch (Exception ex) when (ex is HttpRequestException || ex is TaskCanceledException || ex is InvalidOperationException)
        {
            return new AdoptionCheckResult(
                "environment-feed",
                "Package feed reachability",
                AdoptionStatus.EnvironmentBlocked,
                $"Feed probe failed ({ex.GetType().Name}).",
                $"Rerun with --feed-url {feedUrl} after restoring network access; source checks are unaffected.");
        }
    }

    private static AdoptionCheckResult CheckDocker(TimeSpan timeout)
    {
        try
        {
            var start = new ProcessStartInfo("docker", "info")
            {
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
            };
            using var process = Process.Start(start);
            if (process is null)
            {
                return DockerBlocked("Docker process could not start.");
            }

            if (!process.WaitForExit((int)timeout.TotalMilliseconds))
            {
                try
                {
                    process.Kill(entireProcessTree: true);
                }
                catch (InvalidOperationException)
                {
                }

                return DockerBlocked("Docker probe timed out.");
            }

            return process.ExitCode == 0
                ? new AdoptionCheckResult(
                    "environment-docker",
                    "Docker availability",
                    AdoptionStatus.Pass,
                    "Docker daemon is reachable.",
                    string.Empty)
                : DockerBlocked("Docker daemon is unreachable.");
        }
        catch (Exception ex) when (ex is System.ComponentModel.Win32Exception || ex is InvalidOperationException || ex is PlatformNotSupportedException)
        {
            return DockerBlocked($"Docker probe failed ({ex.GetType().Name}).");
        }
    }

    private static AdoptionCheckResult DockerBlocked(string evidence)
    {
        return new AdoptionCheckResult(
            "environment-docker",
            "Docker availability",
            AdoptionStatus.EnvironmentBlocked,
            evidence,
            "Rerun with environment checks after starting the Docker daemon (docker info); source checks are unaffected.");
    }
}
