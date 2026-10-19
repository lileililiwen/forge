using System.Diagnostics;
using System.Text;

namespace Platform.Adoption.Tests;

/// <summary>
/// CLI-level coverage for exit codes, target reporting, and read-only
/// behavior. Runs the built tool assembly in-process-hosted child runs.
/// </summary>
public sealed class AdoptionToolTests
{
    private static readonly TimeSpan StepTimeout = TimeSpan.FromMinutes(2);

    [Fact]
    public void Doctor_json_reports_target_with_success_exit()
    {
        var run = ExecuteTool($"doctor --project-dir \"{Fixture("adopted")}\" --json");

        Assert.Equal(0, run.ExitCode);
        Assert.Contains(Fixture("adopted"), run.Output, StringComparison.Ordinal);
    }

    [Fact]
    public void Conformance_fails_misconfigured_target()
    {
        var run = ExecuteTool($"conformance --project-dir \"{Fixture("misconfigured")}\"");

        Assert.Equal(1, run.ExitCode);
        Assert.Contains($"Target: {Fixture("misconfigured")}", run.Output, StringComparison.Ordinal);
        Assert.Contains("FAIL platform-pinning", run.Output, StringComparison.Ordinal);
        Assert.Contains("FAIL test-boundary", run.Output, StringComparison.Ordinal);
    }

    [Fact]
    public void Preview_lists_proposals_and_never_writes()
    {
        var fixture = Fixture("misconfigured");
        var before = Snapshot(fixture);
        var run = ExecuteTool($"preview --project-dir \"{fixture}\"");

        Assert.Equal(1, run.ExitCode);
        Assert.Contains("Platform.Core 0.1.* -> 0.1.0", run.Output, StringComparison.Ordinal);
        Assert.Contains("no files were modified", run.Output, StringComparison.OrdinalIgnoreCase);
        Assert.Equal(before, Snapshot(fixture));
    }

    [Theory]
    [InlineData("bogus --project-dir \"/tmp\"")]
    [InlineData("doctor")]
    [InlineData("doctor --project-dir relative/path")]
    public void Usage_errors_exit_64(string arguments)
    {
        var run = ExecuteTool(arguments);

        Assert.Equal(64, run.ExitCode);
    }

    private static (int ExitCode, string Output) ExecuteTool(string arguments)
    {
        var tool = LocateToolAssembly();
        var start = new ProcessStartInfo("dotnet", $"\"{tool}\" {arguments}")
        {
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
        };
        start.Environment["DOTNET_CLI_TELEMETRY_OPTOUT"] = "1";
        start.Environment["DOTNET_NOLOGO"] = "1";
        using var process = Process.Start(start)
            ?? throw new Xunit.Sdk.XunitException("Failed to start the adoption tool.");
        var output = new StringBuilder();
        process.OutputDataReceived += (_, args) => { if (args.Data is not null) output.AppendLine(args.Data); };
        process.ErrorDataReceived += (_, args) => { if (args.Data is not null) output.AppendLine(args.Data); };
        process.BeginOutputReadLine();
        process.BeginErrorReadLine();
        if (!process.WaitForExit((int)StepTimeout.TotalMilliseconds))
        {
            try
            {
                process.Kill(entireProcessTree: true);
            }
            catch (InvalidOperationException)
            {
            }

            throw new Xunit.Sdk.XunitException("Adoption tool timed out.");
        }

        return (process.ExitCode, output.ToString());
    }

    private static string LocateToolAssembly()
    {
        var current = new DirectoryInfo(AppContext.BaseDirectory);
        while (current is not null)
        {
            var tool = Path.Combine(current.FullName, "tools", "Platform.Adoption.Tool", "bin", "Release", "net10.0", "Platform.Adoption.Tool.dll");
            if (File.Exists(tool))
            {
                return tool;
            }

            var debug = tool.Replace("Release", "Debug");
            if (File.Exists(debug))
            {
                return debug;
            }

            current = current.Parent;
        }

        throw new Xunit.Sdk.XunitException("Could not locate the built adoption tool assembly.");
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
        using var hash = System.Security.Cryptography.SHA256.Create();
        var combined = new StringBuilder();
        foreach (var file in Directory.EnumerateFiles(directory, "*", SearchOption.AllDirectories)
                     .OrderBy(path => path, StringComparer.Ordinal))
        {
            combined.Append(file);
            combined.Append(Convert.ToHexString(hash.ComputeHash(File.ReadAllBytes(file))));
        }

        return combined.ToString();
    }
}
