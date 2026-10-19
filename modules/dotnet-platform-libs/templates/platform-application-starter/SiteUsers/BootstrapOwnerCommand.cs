using Microsoft.AspNetCore.Identity;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using StarterApp.SiteUsers;

namespace StarterApp.SiteUsers;

/// <summary>
/// Application-owned Development-only bootstrap-owner command. Generates a
/// random one-time password, hashes it through ASP.NET Identity, and prints
/// the password once. Refuses to run in Production.
/// </summary>
public static class BootstrapOwnerCommand
{
    /// <summary>Runs the bootstrap-owner command. Returns the process exit code.</summary>
    public static async Task<int> RunAsync(string[] args, IHostEnvironment environment, IConfiguration configuration)
    {
        ArgumentNullException.ThrowIfNull(environment);
        ArgumentNullException.ThrowIfNull(configuration);

        if (environment.IsProduction())
        {
            Console.Error.WriteLine("bootstrap-owner is disabled in Production.");
            return 76;
        }

        var email = ParseEmail(args);
        if (string.IsNullOrWhiteSpace(email))
        {
            Console.Error.WriteLine("--email is required.");
            return 64;
        }

        var services = new ServiceCollection();
        services.AddSingleton<IConfiguration>(configuration);
        services.AddSingleton<IHostEnvironment>(environment);
        services.AddLogging();
        services.AddDbContext<ApplicationIdentityDbContext>((provider, options) =>
            options.UseSqlite(configuration.GetConnectionString("Default") ?? "Data Source=app.db"));
        services.AddIdentityCore<ApplicationUser>(options =>
            {
                options.Password.RequireDigit = true;
                options.Password.RequireLowercase = true;
                options.Password.RequiredLength = 8;
            })
            .AddEntityFrameworkStores<ApplicationIdentityDbContext>();

        await using var provider = services.BuildServiceProvider();
        await using var scope = provider.CreateAsyncScope();
        var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationIdentityDbContext>();
        await dbContext.Database.MigrateAsync();
        var userManager = scope.ServiceProvider.GetRequiredService<UserManager<ApplicationUser>>();

        var existing = await userManager.FindByEmailAsync(email);
        if (existing is not null)
        {
            Console.Error.WriteLine($"An owner with email '{email}' already exists. Refusing to overwrite.");
            return 75;
        }

        var password = GenerateRandomPassword();
        var user = new ApplicationUser
        {
            UserName = email,
            Email = email,
            EmailConfirmed = true,
        };
        var result = await userManager.CreateAsync(user, password);
        if (!result.Succeeded)
        {
            Console.Error.WriteLine("Failed to create owner: " + string.Join(", ", result.Errors.Select(e => e.Description)));
            return 1;
        }

        // Print once, never logged, never persisted in plaintext.
        Console.WriteLine($"Owner created for {email}.");
        Console.WriteLine($"One-time password (print this once and store securely): {password}");
        return 0;
    }

    /// <summary>Returns true if the supplied args target the bootstrap-owner command.</summary>
    public static bool IsBootstrapOwnerCommand(string[] args) =>
        args.Length > 0 && string.Equals(args[0], "bootstrap-owner", StringComparison.OrdinalIgnoreCase);

    private static string? ParseEmail(string[] args)
    {
        for (var i = 0; i < args.Length - 1; i++)
        {
            if (string.Equals(args[i], "--email", StringComparison.OrdinalIgnoreCase))
            {
                return args[i + 1];
            }
        }
        return null;
    }

    private static string GenerateRandomPassword()
    {
        const string allowed = "ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789!@#$%^&*?";
        Span<byte> buffer = stackalloc byte[16];
        System.Security.Cryptography.RandomNumberGenerator.Fill(buffer);
        var characters = new char[16];
        for (var i = 0; i < 16; i++)
        {
            characters[i] = allowed[buffer[i] % allowed.Length];
        }
        return new string(characters);
    }
}
