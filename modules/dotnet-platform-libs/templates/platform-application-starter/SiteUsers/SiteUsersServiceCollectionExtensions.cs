using Microsoft.AspNetCore.Authentication.Cookies;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Identity;
using Microsoft.AspNetCore.Identity.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Platform.Authorization;

namespace StarterApp.SiteUsers;

/// <summary>
/// Application-owned registration helpers for the site-user feature. Each
/// generated app wires this in once and owns the resulting DbContext,
/// user store, cookie settings, permission catalog, and Razor pages.
/// </summary>
public static class SiteUsersServiceCollectionExtensions
{
    /// <summary>Registers Identity, cookies, the permission catalog, and razor pages for site users.</summary>
    public static IServiceCollection AddSiteUsers(this IServiceCollection services, IConfiguration configuration)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configuration);

        services.AddOptions<SiteUsersOptions>()
            .Bind(configuration.GetSection(SiteUsersOptions.SectionName))
            .PostConfigure(options =>
            {
                var cookieSection = configuration.GetSection(SiteUsersOptions.SectionName).GetSection(SiteUsersOptions.CookieSectionName);
                if (cookieSection.Exists())
                {
                    options.Cookie = new SiteUsersCookieOptions();
                    cookieSection.Bind(options.Cookie);
                }
            })
            .Validate(options => options.Validate().Count == 0, "Site users options are invalid.")
            .ValidateOnStart();

        services.AddDbContext<ApplicationIdentityDbContext>((provider, options) =>
            options.UseSqlite(configuration.GetConnectionString("Default") ?? "Data Source=app.db"));

        services.AddIdentity<ApplicationUser, IdentityRole>(options =>
            {
                options.SignIn.RequireConfirmedAccount = false;
                options.Password.RequireDigit = true;
                options.Password.RequireLowercase = true;
                options.Password.RequireUppercase = false;
                options.Password.RequireNonAlphanumeric = false;
                options.Password.RequiredLength = 8;
                options.Lockout.DefaultLockoutTimeSpan = TimeSpan.FromMinutes(5);
                options.Lockout.MaxFailedAccessAttempts = 5;
            })
            .AddEntityFrameworkStores<ApplicationIdentityDbContext>()
            .AddDefaultTokenProviders();

        // Configure the application-owned cookie scheme for the application scheme.
        services.ConfigureApplicationCookie(options =>
        {
            var siteOptions = configuration.GetSection(SiteUsersOptions.SectionName).Get<SiteUsersOptions>() ?? new SiteUsersOptions();
            if (!string.IsNullOrWhiteSpace(siteOptions.Cookie.Name))
                options.Cookie.Name = siteOptions.Cookie.Name;
            options.Cookie.SecurePolicy = ParseSecurePolicy(siteOptions.Cookie.SecurePolicy);
            options.Cookie.SameSite = ParseSameSite(siteOptions.Cookie.SameSite);
            options.ExpireTimeSpan = TimeSpan.FromMinutes(siteOptions.Cookie.LifetimeMinutes <= 0 ? 60 : siteOptions.Cookie.LifetimeMinutes);
            options.LoginPath = new PathString(siteOptions.Cookie.LoginPath);
            options.AccessDeniedPath = new PathString(siteOptions.Cookie.AccessDeniedPath);
            options.LogoutPath = new PathString(siteOptions.Cookie.LogoutPath);
            options.ReturnUrlParameter = siteOptions.Cookie.ReturnUrlParameter;
        });

        services.AddSingleton<PermissionCatalog>(provider =>
        {
            var catalog = new PermissionCatalog();
            SiteUserPermissionCatalog.Register(catalog);
            return catalog;
        });

        services.AddAuthorizationBuilder()
            .AddPolicy(PlatformPolicyNames.ForPermission(SiteUserPermissionCatalog.ProfileRead),
                policy => policy.RequireAuthenticatedUser())
            .AddPolicy(PlatformPolicyNames.ForPermission(SiteUserPermissionCatalog.ProfileUpdate),
                policy => policy.RequireAuthenticatedUser());

        return services;
    }

    /// <summary>Wires the cookie authentication scheme and minimal middleware hooks.</summary>
    public static IApplicationBuilder UseSiteUsers(this IApplicationBuilder app)
    {
        ArgumentNullException.ThrowIfNull(app);
        var options = app.ApplicationServices.GetRequiredService<IOptions<SiteUsersOptions>>().Value;
        if (!options.Enabled)
        {
            return app;
        }

        var environment = app.ApplicationServices.GetRequiredService<IHostEnvironment>();
        var failures = options.ValidateProduction(environment);
        if (failures.Count > 0)
        {
            throw new InvalidOperationException("Site users startup validation failed: " + string.Join("; ", failures));
        }

        // Apply pending migrations so a fresh generated app boots with the
        // application-owned identity schema. Production deployments can
        // disable this by running `dotnet ef database update` themselves and
        // setting `SiteUsers:AutoMigrate=false`.
        var configuration = app.ApplicationServices.GetService<IConfiguration>();
        if (configuration?.GetValue("SiteUsers:AutoMigrate", true) == true)
        {
            using var scope = app.ApplicationServices.CreateScope();
            var dbContext = scope.ServiceProvider.GetService<ApplicationIdentityDbContext>();
            if (dbContext is not null && dbContext.Database.GetPendingMigrations().Any())
            {
                dbContext.Database.Migrate();
            }
        }

        app.UseAuthentication();
        app.UseAuthorization();
        return app;
    }

    /// <summary>Returns the documented list of configuration failures for the site-user feature.</summary>
    public static IReadOnlyList<string> Validate(this SiteUsersOptions options)
    {
        var failures = new List<string>();
        if (string.IsNullOrWhiteSpace(options.AuthenticationScheme))
            failures.Add("SiteUsers:AuthenticationScheme is required.");
        if (options.Cookie is null)
            failures.Add("SiteUsers:Cookie section is required.");
        else
        {
            if (string.IsNullOrWhiteSpace(options.Cookie.SecurePolicy))
                failures.Add("SiteUsers:Cookie:SecurePolicy is required.");
            else if (!IsAllowedSecurePolicy(options.Cookie.SecurePolicy))
                failures.Add($"SiteUsers:Cookie:SecurePolicy '{options.Cookie.SecurePolicy}' is not recognized. Allowed: Always, SameAsRequest, None.");
            if (options.Cookie.LifetimeMinutes <= 0)
                failures.Add("SiteUsers:Cookie:LifetimeMinutes must be positive.");
            if (string.IsNullOrWhiteSpace(options.Cookie.LoginPath))
                failures.Add("SiteUsers:Cookie:LoginPath is required.");
            if (string.IsNullOrWhiteSpace(options.Cookie.LogoutPath))
                failures.Add("SiteUsers:Cookie:LogoutPath is required.");
        }
        return failures;
    }

    /// <summary>Returns the documented production-time validation list.</summary>
    public static IReadOnlyList<string> ValidateProduction(this SiteUsersOptions options, IHostEnvironment environment)
    {
        var failures = new List<string>();
        if (!options.Enabled)
            return failures;
        if (!environment.IsProduction())
            return failures;
        if (options.DevelopmentOnly)
            failures.Add("SiteUsers:DevelopmentOnly must be false in Production.");
        if (options.Cookie is null || !string.Equals(options.Cookie.SecurePolicy, "Always", StringComparison.OrdinalIgnoreCase))
            failures.Add("SiteUsers:Cookie:SecurePolicy must be 'Always' in Production.");
        if (string.IsNullOrWhiteSpace(options.Cookie?.Name))
            failures.Add("SiteUsers:Cookie:Name is required in Production.");
        return failures;
    }

    private static bool IsAllowedSecurePolicy(string value) =>
        value.Equals("Always", StringComparison.OrdinalIgnoreCase)
        || value.Equals("SameAsRequest", StringComparison.OrdinalIgnoreCase)
        || value.Equals("None", StringComparison.OrdinalIgnoreCase);

    private static CookieSecurePolicy ParseSecurePolicy(string value) => value?.ToLowerInvariant() switch
    {
        "always" => CookieSecurePolicy.Always,
        "none" => CookieSecurePolicy.None,
        _ => CookieSecurePolicy.SameAsRequest,
    };

    private static SameSiteMode ParseSameSite(string value) => value?.ToLowerInvariant() switch
    {
        "strict" => SameSiteMode.Strict,
        "none" => SameSiteMode.None,
        _ => SameSiteMode.Lax,
    };
}
