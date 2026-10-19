using System.Diagnostics;
using System.Text;

namespace Platform.Template.Tests;

/// <summary>
/// Packs the platform solution plus the template pack into a private local
/// feed once per run and installs the template. Heavy by design: installing
/// and generating are the behaviors under test. All template tests share the
/// serial <c>TemplatePack</c> collection so install state never races.
/// </summary>
public sealed class TemplatePackFixture : IDisposable
{
    private const string TemplatePackageId = "Platform.Application.Template";
    private const string TemplateVersion = "0.1.0";
    private const string MirrorSource = "https://repo.huaweicloud.com/repository/nuget/v3/index.json";

    public TemplatePackFixture()
    {
        RepositoryRoot = LocateRepositoryRoot();
        WorkRoot = Path.Combine(Path.GetTempPath(), $"platform-template-{Guid.NewGuid():N}");
        FeedDirectory = Path.Combine(WorkRoot, "feed");
        Directory.CreateDirectory(FeedDirectory);

        // Pin the repository SDK for every child dotnet invocation. The repo
        // pins its SDK through global.json, but fixture children run with a
        // temp working directory where that pin is invisible and a newer
        // machine-wide SDK would otherwise be selected.
        var repositoryGlobalJson = Path.Combine(RepositoryRoot, "global.json");
        if (File.Exists(repositoryGlobalJson))
        {
            File.Copy(repositoryGlobalJson, Path.Combine(WorkRoot, "global.json"));
        }

        RunDotnet(
            $"pack \"{Path.Combine(RepositoryRoot, "Platform.sln")}\" -c Release -o \"{FeedDirectory}\" --nologo -m:1",
            RepositoryRoot,
            TimeSpan.FromMinutes(20));
        RunDotnet(
            $"pack \"{Path.Combine(RepositoryRoot, "templates", "Platform.Application.Template", "Platform.Application.Template.csproj")}\" -c Release -o \"{FeedDirectory}\" --nologo",
            RepositoryRoot,
            TimeSpan.FromMinutes(10));

        TemplatePackagePath = Path.Combine(FeedDirectory, $"{TemplatePackageId}.{TemplateVersion}.nupkg");
        if (!File.Exists(TemplatePackagePath))
        {
            throw new Xunit.Sdk.XunitException($"Template package was not packed to {TemplatePackagePath}.");
        }

        RunDotnet($"new install \"{TemplatePackagePath}\"", WorkRoot, TimeSpan.FromMinutes(5));
    }

    public string RepositoryRoot { get; }

    public string WorkRoot { get; }

    public string FeedDirectory { get; }

    public string TemplatePackagePath { get; }

    public static string Mirror => MirrorSource;

    public void ReinstallTemplate()
    {
        RunDotnet($"new install \"{TemplatePackagePath}\"", WorkRoot, TimeSpan.FromMinutes(5));
    }

    public void UninstallTemplate()
    {
        RunDotnet($"new uninstall {TemplatePackageId}", WorkRoot, TimeSpan.FromMinutes(5));
    }

    public void Dispose()
    {
        try
        {
            RunDotnet($"new uninstall {TemplatePackageId}", WorkRoot, TimeSpan.FromMinutes(5));
        }
        catch (Xunit.Sdk.XunitException)
        {
            // Best effort: uninstalling twice (after the detachment test) must not fail the run.
        }

        try
        {
            Directory.Delete(WorkRoot, recursive: true);
        }
        catch (IOException)
        {
            // Best effort: a locked temp file must not fail the suite.
        }
    }

    public static void RunDotnet(string arguments, string workingDirectory, TimeSpan timeout)
    {
        var start = new ProcessStartInfo("dotnet", arguments)
        {
            WorkingDirectory = workingDirectory,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
        };
        start.Environment["DOTNET_CLI_TELEMETRY_OPTOUT"] = "1";
        start.Environment["DOTNET_SKIP_FIRST_TIME_EXPERIENCE"] = "1";
        start.Environment["DOTNET_NOLOGO"] = "1";

        using var process = Process.Start(start)
            ?? throw new Xunit.Sdk.XunitException($"Failed to start dotnet {arguments}.");
        var output = new StringBuilder();
        var error = new StringBuilder();
        process.OutputDataReceived += (_, args) => { if (args.Data is not null) output.AppendLine(args.Data); };
        process.ErrorDataReceived += (_, args) => { if (args.Data is not null) error.AppendLine(args.Data); };
        process.BeginOutputReadLine();
        process.BeginErrorReadLine();
        if (!process.WaitForExit((int)timeout.TotalMilliseconds))
        {
            try
            {
                process.Kill(entireProcessTree: true);
            }
            catch (InvalidOperationException)
            {
            }

            throw new Xunit.Sdk.XunitException($"dotnet {arguments} timed out after {timeout}. Output: {Truncate(output.ToString())}");
        }

        if (process.ExitCode != 0)
        {
            throw new Xunit.Sdk.XunitException(
                $"dotnet {arguments} exited {process.ExitCode} in {workingDirectory}. Output: {Truncate(output.ToString())} Error: {Truncate(error.ToString())}");
        }
    }

    private static string Truncate(string value)
    {
        const int limit = 4000;
        return value.Length <= limit ? value : value[^limit..];
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

        throw new Xunit.Sdk.XunitException("Could not locate the platform repository root.");
    }
}
