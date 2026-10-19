using Hangfire.Dashboard;

namespace Platform.Jobs.Hangfire;

/// <summary>
/// Dashboard authorization filter that delegates every decision to the
/// application-provided callback. The platform never ships a default
/// authorization policy, credentials, or an open fallback: without a
/// callback the dashboard cannot be mapped at all.
/// </summary>
public sealed class CallbackDashboardAuthorizationFilter : IDashboardAuthorizationFilter
{
    private readonly Func<DashboardContext, bool> _callback;

    /// <summary>
    /// Initializes a new instance of the <see cref="CallbackDashboardAuthorizationFilter"/> class.
    /// </summary>
    /// <param name="callback">The application-provided authorization callback.</param>
    public CallbackDashboardAuthorizationFilter(Func<DashboardContext, bool> callback)
    {
        ArgumentNullException.ThrowIfNull(callback);
        _callback = callback;
    }

    /// <inheritdoc />
    public bool Authorize(DashboardContext context)
    {
        ArgumentNullException.ThrowIfNull(context);
        return _callback(context);
    }
}
