namespace Platform.Template.Tests;

[CollectionDefinition("TemplatePack", DisableParallelization = true)]
public sealed class TemplatePackCollection
{
}

[Collection("TemplatePack")]
public class TemplatePackSmokeTests : IClassFixture<TemplatePackFixture>
{
    private static readonly TimeSpan StepTimeout = TimeSpan.FromMinutes(10);
    private readonly TemplatePackFixture _fixture;

    public TemplatePackSmokeTests(TemplatePackFixture fixture)
    {
        _fixture = fixture;
    }

    public static TheoryData<bool, bool, bool> Variants()
    {
        var data = new TheoryData<bool, bool, bool>();
        foreach (var includeTests in new[] { true, false })
        {
            foreach (var enableIdentity in new[] { true, false })
            {
                foreach (var enablePersistence in new[] { true, false })
                {
                    data.Add(includeTests, enableIdentity, enablePersistence);
                }
            }
        }

        return data;
    }

    [Theory]
    [MemberData(nameof(Variants))]
    public void Generated_variant_restores_builds_and_tests(
        bool includeTests, bool enableIdentity, bool enablePersistence)
    {
        var appName = $"Sample{(includeTests ? "T" : "F")}{(enableIdentity ? "I" : "N")}{(enablePersistence ? "P" : "N")}";
        var appDirectory = Generate(appName, includeTests, enableIdentity, enablePersistence);

        AssertGeneratedTree(appDirectory, appName, includeTests, enableIdentity, enablePersistence);
        TemplatePackFixture.RunDotnet("build -c Release --nologo", appDirectory, StepTimeout);
        if (includeTests)
        {
            TemplatePackFixture.RunDotnet("test -c Release --no-build --nologo", appDirectory, StepTimeout);
        }
    }

    [Fact]
    public void Generated_application_survives_template_uninstall()
    {
        var appDirectory = Generate("Detached", includeTests: true, enableIdentity: false, enablePersistence: false);
        try
        {
            _fixture.UninstallTemplate();
            TemplatePackFixture.RunDotnet("build -c Release --nologo", appDirectory, StepTimeout);
            TemplatePackFixture.RunDotnet("test -c Release --no-build --nologo", appDirectory, StepTimeout);
        }
        finally
        {
            _fixture.ReinstallTemplate();
        }
    }

    private string Generate(string appName, bool includeTests, bool enableIdentity, bool enablePersistence)
    {
        var appDirectory = Path.Combine(_fixture.WorkRoot, appName);
        if (Directory.Exists(appDirectory))
        {
            Directory.Delete(appDirectory, recursive: true);
        }

        TemplatePackFixture.RunDotnet(
            $"new platform-app -n {appName} -o \"{appDirectory}\" --IncludeTests {includeTests} --EnableIdentity {enableIdentity} --EnablePersistence {enablePersistence}",
            _fixture.WorkRoot,
            StepTimeout);
        File.WriteAllText(
            Path.Combine(appDirectory, "NuGet.config"),
            $"""
            <?xml version="1.0" encoding="utf-8"?>
            <configuration>
              <packageSources>
                <clear />
                <add key="local" value="{_fixture.FeedDirectory}" />
                <add key="mirror" value="{TemplatePackFixture.Mirror}" />
              </packageSources>
            </configuration>
            """);
        return appDirectory;
    }

    private static void AssertGeneratedTree(
        string appDirectory, string appName, bool includeTests, bool enableIdentity, bool enablePersistence)
    {
        Assert.True(File.Exists(Path.Combine(appDirectory, $"{appName}.csproj")), "App project must be generated.");
        Assert.True(File.Exists(Path.Combine(appDirectory, "Program.cs")), "Program.cs must be generated.");
        Assert.True(File.Exists(Path.Combine(appDirectory, "appsettings.json")), "appsettings.json must be generated.");
        Assert.False(Directory.Exists(Path.Combine(appDirectory, ".template.config")), "Template metadata must not leak into generated source.");

        var testProject = Path.Combine(appDirectory, "tests", $"{appName}.Tests", $"{appName}.Tests.csproj");
        Assert.Equal(includeTests, File.Exists(testProject));

        var contextFile = Path.Combine(appDirectory, "Data", "AppDbContext.cs");
        Assert.Equal(enablePersistence, File.Exists(contextFile));

        var program = File.ReadAllText(Path.Combine(appDirectory, "Program.cs"));
        Assert.DoesNotContain("//#if", program, StringComparison.Ordinal);
        Assert.DoesNotContain("//#endif", program, StringComparison.Ordinal);
        Assert.Equal(enableIdentity, program.Contains("options.EnableIdentity = true;", StringComparison.Ordinal));
        Assert.Equal(enablePersistence, program.Contains("AddDbContext<AppDbContext>", StringComparison.Ordinal));

        var project = File.ReadAllText(Path.Combine(appDirectory, $"{appName}.csproj"));
        Assert.DoesNotContain("<!--#if", project, StringComparison.Ordinal);
        Assert.Equal(
            enablePersistence,
            project.Contains("Platform.Persistence.EfCore", StringComparison.Ordinal));
        foreach (var file in Directory.EnumerateFiles(appDirectory, "*", SearchOption.AllDirectories))
        {
            if (file.EndsWith("NuGet.config", StringComparison.OrdinalIgnoreCase)
                || file.EndsWith("README.md", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            var content = File.ReadAllText(file);
            Assert.DoesNotContain("Platform.Application.Template", content, StringComparison.Ordinal);
        }
    }
}
