using System.Text;

namespace Platform.Template.Tests;

[Collection("TemplatePack")]
public sealed class SiteUserAuthTemplateTests : IClassFixture<TemplatePackFixture>
{
    private static readonly TimeSpan StepTimeout = TimeSpan.FromMinutes(10);
    private readonly TemplatePackFixture _fixture;

    public SiteUserAuthTemplateTests(TemplatePackFixture fixture)
    {
        _fixture = fixture;
    }

    [Fact]
    public void EnableSiteUsers_true_renders_full_feature_tree()
    {
        var appName = "SiteUsersOn";
        var appDirectory = Generate(appName, enableSiteUsers: true);

        // Feature implementation files must all be present.
        Assert.True(File.Exists(Path.Combine(appDirectory, "SiteUsers", "ApplicationUser.cs")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "SiteUsers", "ApplicationIdentityDbContext.cs")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "SiteUsers", "SiteUsersOptions.cs")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "SiteUsers", "SiteUserPermissionCatalog.cs")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "SiteUsers", "SiteUsersServiceCollectionExtensions.cs")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "SiteUsers", "BootstrapOwnerCommand.cs")));
        Assert.True(Directory.Exists(Path.Combine(appDirectory, "SiteUsers", "Migrations")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "SiteUsers", "Migrations", "20261001060926_InitialIdentitySchema.cs")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "SiteUsers", "Migrations", "20261001060926_InitialIdentitySchema.Designer.cs")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "SiteUsers", "Migrations", "ApplicationIdentityDbContextModelSnapshot.cs")));

        // Razor pages must be present.
        Assert.True(File.Exists(Path.Combine(appDirectory, "Pages", "Identity", "Account", "Login.cshtml")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "Pages", "Identity", "Account", "Login.cshtml.cs")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "Pages", "Identity", "Account", "Logout.cshtml")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "Pages", "Identity", "Account", "Logout.cshtml.cs")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "Pages", "Identity", "Account", "ForgotPassword.cshtml")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "Pages", "Identity", "Account", "ForgotPassword.cshtml.cs")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "Pages", "Identity", "Account", "AccessDenied.cshtml")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "Pages", "Shared", "_Layout.cshtml")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "Pages", "Shared", "_ViewImports.cshtml")));
        Assert.True(File.Exists(Path.Combine(appDirectory, "Pages", "Shared", "_ViewStart.cshtml")));

        // Self-registration: the page, its code-behind, the opt-in it reads, and the
        // antiforgery token its form carries.
        var registerPage = Path.Combine(appDirectory, "Pages", "Identity", "Account", "Register.cshtml");
        var registerCodeBehind = registerPage + ".cs";
        Assert.True(File.Exists(registerPage));
        Assert.True(File.Exists(registerCodeBehind));
        var registerMarkup = File.ReadAllText(registerPage);
        Assert.Contains("@Html.AntiForgeryToken()", registerMarkup, StringComparison.Ordinal);
        Assert.Contains("Model.IsRegistrationEnabled", registerMarkup, StringComparison.Ordinal);
        var registerModel = File.ReadAllText(registerCodeBehind);
        Assert.Contains("SelfRegistrationEnabled", registerModel, StringComparison.Ordinal);
        Assert.Contains("UserManager<ApplicationUser>", registerModel, StringComparison.Ordinal);
        // The generated appsettings keeps the opt-in off until the owner turns it on.
        Assert.Contains("\"SelfRegistrationEnabled\": false", File.ReadAllText(Path.Combine(appDirectory, "appsettings.json")), StringComparison.Ordinal);

        // No separate tools/BootstrapOwner project — bootstrap-owner runs in-process.
        Assert.False(Directory.Exists(Path.Combine(appDirectory, "tools")));

        // Generated csproj must reference the right platform packages.
        var project = File.ReadAllText(Path.Combine(appDirectory, $"{appName}.csproj"));
        Assert.Contains("Microsoft.AspNetCore.Identity.EntityFrameworkCore", project, StringComparison.Ordinal);
        Assert.Contains("Microsoft.EntityFrameworkCore.Sqlite", project, StringComparison.Ordinal);
        Assert.Contains("Platform.Identity.AspNetCore", project, StringComparison.Ordinal);
        Assert.Contains("Platform.Authorization", project, StringComparison.Ordinal);

        // Program.cs must wire the feature and the bootstrap-owner dispatch.
        var program = File.ReadAllText(Path.Combine(appDirectory, "Program.cs"));
        Assert.Contains("AddPlatformIdentity", program, StringComparison.Ordinal);
        Assert.Contains("AddSiteUsers", program, StringComparison.Ordinal);
        Assert.Contains("UseSiteUsers", program, StringComparison.Ordinal);
        Assert.Contains("AddRazorPages", program, StringComparison.Ordinal);
        Assert.Contains("MapRazorPages", program, StringComparison.Ordinal);
        Assert.Contains("BootstrapOwnerCommand", program, StringComparison.Ordinal);

        // appsettings.json must include the SiteUsers section.
        var appsettings = File.ReadAllText(Path.Combine(appDirectory, "appsettings.json"));
        Assert.Contains("SiteUsers", appsettings, StringComparison.Ordinal);

        // The test project must reference Platform.Identity.Testing.
        var testProjectPath = Path.Combine(appDirectory, "tests", $"{appName}.Tests", $"{appName}.Tests.csproj");
        var testProject = File.ReadAllText(testProjectPath);
        Assert.Contains("Platform.Identity.Testing", testProject, StringComparison.Ordinal);

        // The site-user test file must be present.
        var siteUserTestsPath = Path.Combine(appDirectory, "tests", $"{appName}.Tests", "SiteUserAuthTests.cs");
        Assert.True(File.Exists(siteUserTestsPath));
    }

    [Fact]
    public void EnableSiteUsers_false_omits_identity_feature_completely()
    {
        var appName = "SiteUsersOff";
        var appDirectory = Generate(appName, enableSiteUsers: false);

        // No SiteUsers / Pages / Migrations files.
        Assert.False(Directory.Exists(Path.Combine(appDirectory, "SiteUsers")));
        Assert.False(Directory.Exists(Path.Combine(appDirectory, "Pages")));

        // The Register page rides on the existing Pages/Identity/** exclude, so a
        // disabled template emits no self-registration surface at all.
        Assert.False(File.Exists(Path.Combine(appDirectory, "Pages", "Identity", "Account", "Register.cshtml")));
        Assert.False(File.Exists(Path.Combine(appDirectory, "Pages", "Identity", "Account", "Register.cshtml.cs")));

        // Generated csproj must NOT reference any Identity or Razor Pages packages.
        var project = File.ReadAllText(Path.Combine(appDirectory, $"{appName}.csproj"));
        Assert.DoesNotContain("Microsoft.AspNetCore.Identity", project, StringComparison.Ordinal);
        Assert.DoesNotContain("Microsoft.EntityFrameworkCore.Sqlite", project, StringComparison.Ordinal);
        Assert.DoesNotContain("Platform.Identity.AspNetCore", project, StringComparison.Ordinal);

        // Program.cs must not wire site-user APIs.
        var program = File.ReadAllText(Path.Combine(appDirectory, "Program.cs"));
        Assert.DoesNotContain("AddSiteUsers", program, StringComparison.Ordinal);
        Assert.DoesNotContain("UseSiteUsers", program, StringComparison.Ordinal);
        Assert.DoesNotContain("BootstrapOwnerCommand", program, StringComparison.Ordinal);
        Assert.DoesNotContain("AddRazorPages", program, StringComparison.Ordinal);
        Assert.DoesNotContain("MapRazorPages", program, StringComparison.Ordinal);

        // The site-user test file must be omitted.
        var siteUserTestsPath = Path.Combine(appDirectory, "tests", $"{appName}.Tests", "SiteUserAuthTests.cs");
        Assert.False(File.Exists(siteUserTestsPath));
    }

    [Fact]
    public void EnableSiteUsers_true_generated_app_builds_and_runs_tests()
    {
        var appName = "SiteUsersOnTest";
        var appDirectory = Generate(appName, enableSiteUsers: true);

        TemplatePackFixture.RunDotnet("build -c Release --nologo", appDirectory, StepTimeout);
        TemplatePackFixture.RunDotnet("test -c Release --no-build --nologo", appDirectory, StepTimeout);
    }

    [Fact]
    public void EnableSiteUsers_false_generated_app_builds_clean()
    {
        var appName = "SiteUsersOffTest";
        var appDirectory = Generate(appName, enableSiteUsers: false);

        TemplatePackFixture.RunDotnet("build -c Release --nologo", appDirectory, StepTimeout);
    }

    private string Generate(string appName, bool enableSiteUsers)
    {
        var appDirectory = Path.Combine(_fixture.WorkRoot, appName);
        if (Directory.Exists(appDirectory))
        {
            Directory.Delete(appDirectory, recursive: true);
        }

        TemplatePackFixture.RunDotnet(
            $"new platform-app -n {appName} -o \"{appDirectory}\" --EnableSiteUsers {enableSiteUsers}",
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
}
