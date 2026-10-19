using Hangfire;
using Hangfire.Dashboard;

namespace Platform.Jobs.Hangfire;

/// <summary>
/// Builds the Hangfire <see cref="DashboardOptions"/> used by the platform
/// adapter. Storage connection strings are never displayed and the only
/// authorization filter is the application-provided callback.
/// </summary>
public static class HangfireDashboardOptionsFactory
{
    /// <summary>
    /// Creates dashboard options that gate every request through the
    /// supplied <paramref name="authorization"/> callback and never display
    /// the storage connection string.
    /// </summary>
    /// <param name="authorization">The application-provided authorization callback.</param>
    /// <returns>The configured <see cref="DashboardOptions"/>.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="authorization"/> is <c>null</c>.</exception>
    public static DashboardOptions Create(Func<DashboardContext, bool> authorization)
    {
        ArgumentNullException.ThrowIfNull(authorization);

        return new DashboardOptions
        {
            Authorization = new IDashboardAuthorizationFilter[]
            {
                new CallbackDashboardAuthorizationFilter(authorization),
            },
            DisplayStorageConnectionString = false,
        };
    }
}
