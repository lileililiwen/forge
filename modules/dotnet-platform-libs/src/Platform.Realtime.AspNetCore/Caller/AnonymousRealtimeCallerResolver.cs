using Microsoft.AspNetCore.Http;
using Platform.Core.Context;

namespace Platform.Realtime.AspNetCore.Caller;

/// <summary>
/// Default caller resolver that returns <see cref="CallerContext.Anonymous"/>.
/// Hosts must register their own <see cref="IRealtimeCallerResolver"/> to apply
/// tenant or subject scoping; until they do, connections carry no caller context.
/// </summary>
public sealed class AnonymousRealtimeCallerResolver : IRealtimeCallerResolver
{
    /// <inheritdoc />
    public ValueTask<CallerContext> ResolveAsync(HttpContext http, CancellationToken cancellationToken = default) =>
        new(CallerContext.Anonymous);
}
