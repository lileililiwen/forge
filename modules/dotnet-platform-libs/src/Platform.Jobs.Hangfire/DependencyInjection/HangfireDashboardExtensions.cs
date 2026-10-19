using Hangfire;
using Microsoft.AspNetCore.Builder;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;

namespace Platform.Jobs.Hangfire.DependencyInjection;

/// <summary>
/// Opt-in dashboard mapping. The dashboard is never mapped unless the host
/// enabled it in <see cref="HangfireJobsOptions"/> and supplied an
/// application-provided authorization callback; mapping without the
/// callback fails fast instead of serving an unprotected dashboard.
/// </summary>
public static class HangfireDashboardExtensions
{
    /// <summary>
    /// Maps the Hangfire dashboard when <see cref="HangfireJobsOptions.DashboardEnabled"/>
    /// is <c>true</c>; otherwise returns the builder unchanged. The dashboard
    /// is gated by the application-provided authorization callback and never
    /// displays the storage connection string.
    /// </summary>
    /// <param name="app">The application builder.</param>
    /// <returns>The same <paramref name="app"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="app"/> is <c>null</c>.</exception>
    /// <exception cref="InvalidOperationException">The dashboard is enabled without an authorization callback.</exception>
    public static IApplicationBuilder UsePlatformHangfireDashboard(this IApplicationBuilder app)
    {
        ArgumentNullException.ThrowIfNull(app);

        var options = app.ApplicationServices.GetRequiredService<IOptions<HangfireJobsOptions>>().Value;
        return app.UsePlatformHangfireDashboard(options);
    }

    /// <summary>
    /// Maps the Hangfire dashboard using the supplied options when
    /// <see cref="HangfireJobsOptions.DashboardEnabled"/> is <c>true</c>;
    /// otherwise returns the builder unchanged.
    /// </summary>
    /// <param name="app">The application builder.</param>
    /// <param name="options">The validated adapter options.</param>
    /// <param name="storage">The optional storage override; defaults to the storage the adapter registered.</param>
    /// <returns>The same <paramref name="app"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="app"/> or <paramref name="options"/> is <c>null</c>.</exception>
    /// <exception cref="InvalidOperationException">The dashboard is enabled without an authorization callback.</exception>
    public static IApplicationBuilder UsePlatformHangfireDashboard(
        this IApplicationBuilder app,
        HangfireJobsOptions options,
        JobStorage? storage = null)
    {
        ArgumentNullException.ThrowIfNull(app);
        ArgumentNullException.ThrowIfNull(options);

        if (!options.DashboardEnabled)
        {
            return app;
        }

        if (options.DashboardAuthorization is null)
        {
            throw new InvalidOperationException(
                "The Hangfire dashboard is enabled but no authorization callback is configured. Set HangfireJobsOptions.DashboardAuthorization before mapping the dashboard.");
        }

        return app.UseHangfireDashboard(
            options.DashboardRoute,
            HangfireDashboardOptionsFactory.Create(options.DashboardAuthorization),
            storage);
    }
}
