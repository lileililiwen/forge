using Platform.Adoption;

// platform-doctor: read-only adoption diagnostics over an explicit directory.
// Commands: doctor | inventory | conformance | preview
// Exit codes: 0 clean, 1 source failures, 2 environment-blocked only, 64 usage.
return Run(args);

static int Run(string[] args)
{
    if (args.Length == 0 || args[0] is "-h" or "--help")
    {
        PrintUsage();
        return AdoptionExitCodes.Success;
    }

    var command = args[0].ToLowerInvariant();
    if (command is not ("doctor" or "inventory" or "conformance" or "preview"))
    {
        Console.Error.WriteLine($"Unknown command '{args[0]}'. Expected doctor, inventory, conformance, or preview.");
        return AdoptionExitCodes.Usage;
    }

    var options = ParseOptions(args[1..]);
    if (options is null)
    {
        return AdoptionExitCodes.Usage;
    }

    AdoptionReport report;
    try
    {
        report = AdoptionAnalyzer.Analyze(options.ProjectDir, options.Adoption);
    }
    catch (AdoptionTargetException ex)
    {
        Console.Error.WriteLine($"Target error: {ex.Message} (target: '{ex.Target}')");
        return AdoptionExitCodes.Usage;
    }

    return command switch
    {
        "doctor" => WriteDoctor(report, options.Json),
        "inventory" => WriteInventory(report, options.Json),
        "conformance" => WriteConformance(report, options.Json),
        "preview" => WritePreview(report, options.Json),
        _ => AdoptionExitCodes.Usage,
    };
}

static void PrintUsage()
{
    Console.WriteLine("Usage: platform-doctor <doctor|inventory|conformance|preview> --project-dir <absolute-path> [--json] [--include-environment] [--feed-url <url>] [--check-docker] [--expected-platform-version <version>]");
    Console.WriteLine("  doctor       Run all adoption checks against the target directory.");
    Console.WriteLine("  inventory    List discovered projects and package references.");
    Console.WriteLine("  conformance  Run pinning and test-boundary checks only.");
    Console.WriteLine("  preview      List proposed package alignment edits without writing files.");
    Console.WriteLine("Exit codes: 0 clean, 1 source failures, 2 environment-blocked only, 64 usage error.");
}

static ParsedOptions? ParseOptions(string[] args)
{
    string? projectDir = null;
    var json = false;
    var includeEnvironment = false;
    string? feedUrl = null;
    var checkDocker = false;
    var expectedVersion = "0.1.0";

    for (var i = 0; i < args.Length; i++)
    {
        switch (args[i])
        {
            case "--project-dir" when i + 1 < args.Length:
                projectDir = args[++i];
                break;
            case "--json":
                json = true;
                break;
            case "--include-environment":
                includeEnvironment = true;
                break;
            case "--feed-url" when i + 1 < args.Length:
                feedUrl = args[++i];
                break;
            case "--check-docker":
                checkDocker = true;
                break;
            case "--expected-platform-version" when i + 1 < args.Length:
                expectedVersion = args[++i];
                break;
            default:
                Console.Error.WriteLine($"Unknown or incomplete option '{args[i]}'.");
                return null;
        }
    }

    if (string.IsNullOrWhiteSpace(projectDir))
    {
        Console.Error.WriteLine("Missing required --project-dir <absolute-path>.");
        return null;
    }

    return new ParsedOptions(
        projectDir,
        json,
        new AdoptionOptions
        {
            ExpectedPlatformVersion = expectedVersion,
            IncludeEnvironmentChecks = includeEnvironment,
            FeedUrl = feedUrl,
            CheckDocker = checkDocker,
        });
}

static int WriteDoctor(AdoptionReport report, bool json)
{
    if (json)
    {
        Console.WriteLine(AdoptionAnalyzer.ToJson(report));
        return report.ExitCode;
    }

    Console.WriteLine($"Target: {report.TargetDirectory}");
    foreach (var result in report.Results)
    {
        var label = result.Status switch
        {
            AdoptionStatus.Pass => "PASS",
            AdoptionStatus.Warning => "WARN",
            AdoptionStatus.Failed => "FAIL",
            AdoptionStatus.EnvironmentBlocked => "BLOCKED",
            _ => "UNKNOWN",
        };
        Console.WriteLine($"{label} {result.CheckId} — {result.Evidence}");
        if (!string.IsNullOrEmpty(result.Remediation))
        {
            Console.WriteLine($"      Remediation: {result.Remediation}");
        }
    }

    Console.WriteLine($"Summary: {report.Results.Count(r => r.Status == AdoptionStatus.Pass)} passed, " +
        $"{report.Results.Count(r => r.Status == AdoptionStatus.Warning)} warnings, " +
        $"{report.Results.Count(r => r.Status == AdoptionStatus.Failed)} failed, " +
        $"{report.BlockedCount} environment-blocked.");
    return report.ExitCode;
}

static int WriteInventory(AdoptionReport report, bool json)
{
    var projects = AdoptionInventory.ListProjects(report.TargetDirectory);
    if (json)
    {
        Console.WriteLine(AdoptionInventory.ToJson(report.TargetDirectory, projects));
        return report.HasFailures ? AdoptionExitCodes.Failures : AdoptionExitCodes.Success;
    }

    Console.WriteLine($"Target: {report.TargetDirectory}");
    foreach (var project in projects)
    {
        Console.WriteLine($"Project: {project.RelativePath}");
        foreach (var package in project.Packages)
        {
            Console.WriteLine($"  Package: {package.Id} {package.Version}");
        }
    }

    Console.WriteLine($"Summary: {projects.Count} project(s).");
    return report.HasFailures ? AdoptionExitCodes.Failures : AdoptionExitCodes.Success;
}

static int WriteConformance(AdoptionReport report, bool json)
{
    var selected = report.Results
        .Where(r => r.CheckId is "platform-pinning" or "test-boundary")
        .ToList();
    if (json)
    {
        Console.WriteLine(AdoptionAnalyzer.ToJson(report with { Results = selected }));
        return selected.Any(r => r.Status == AdoptionStatus.Failed) ? AdoptionExitCodes.Failures : AdoptionExitCodes.Success;
    }

    Console.WriteLine($"Target: {report.TargetDirectory}");
    foreach (var result in selected)
    {
        var label = result.Status switch
        {
            AdoptionStatus.Pass => "PASS",
            AdoptionStatus.Warning => "WARN",
            AdoptionStatus.Failed => "FAIL",
            AdoptionStatus.EnvironmentBlocked => "BLOCKED",
            _ => "UNKNOWN",
        };
        Console.WriteLine($"{label} {result.CheckId} — {result.Evidence}");
        if (!string.IsNullOrEmpty(result.Remediation))
        {
            Console.WriteLine($"      Remediation: {result.Remediation}");
        }
    }

    return selected.Any(r => r.Status == AdoptionStatus.Failed) ? AdoptionExitCodes.Failures : AdoptionExitCodes.Success;
}

static int WritePreview(AdoptionReport report, bool json)
{
    if (json)
    {
        Console.WriteLine(AdoptionPreview.ToJson(report));
        return report.HasFailures ? AdoptionExitCodes.Failures : AdoptionExitCodes.Success;
    }

    Console.WriteLine($"Target: {report.TargetDirectory}");
    if (report.ProposedEdits.Count == 0)
    {
        Console.WriteLine("No package alignment edits proposed.");
    }

    foreach (var edit in report.ProposedEdits)
    {
        Console.WriteLine($"Propose: {edit.ProjectFile}: {edit.PackageId} {edit.CurrentVersion} -> {edit.ProposedVersion} ({edit.Reason})");
    }

    Console.WriteLine("Preview only: no files were modified.");
    return report.HasFailures ? AdoptionExitCodes.Failures : AdoptionExitCodes.Success;
}

sealed record ParsedOptions(string ProjectDir, bool Json, AdoptionOptions Adoption);
