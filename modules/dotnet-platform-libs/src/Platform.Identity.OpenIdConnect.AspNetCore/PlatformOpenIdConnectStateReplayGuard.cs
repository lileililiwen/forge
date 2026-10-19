using System.Collections.Concurrent;
using System.Security.Cryptography;
using System.Text;

namespace Platform.Identity.OpenIdConnect.AspNetCore;

/// <summary>
/// Rejects a callback state that was already consumed. The adapter calls the guard
/// before the authorization code is exchanged, so a replayed callback never reaches
/// the provider and never produces a second session.
/// </summary>
public interface IPlatformOpenIdConnectStateReplayGuard
{
    /// <summary>
    /// Consumes a callback state. It returns <c>true</c> the first time a state is seen
    /// and <c>false</c> for every later use of the same state.
    /// </summary>
    Task<bool> TryConsumeAsync(string state, CancellationToken cancellationToken);
}

/// <summary>
/// The default per-process replay guard. It stores only a hash of the state, forgets it
/// after the configured lifetime, and is safe to replace with a shared implementation
/// when the application runs on more than one instance.
/// </summary>
public sealed class InMemoryPlatformOpenIdConnectStateReplayGuard : IPlatformOpenIdConnectStateReplayGuard
{
    private readonly ConcurrentDictionary<string, DateTimeOffset> _consumed = new(StringComparer.Ordinal);
    private readonly TimeProvider _timeProvider;
    private readonly TimeSpan _lifetime;

    /// <summary>Creates a guard with the default ten-minute state lifetime.</summary>
    public InMemoryPlatformOpenIdConnectStateReplayGuard()
        : this(TimeProvider.System, TimeSpan.FromMinutes(10))
    {
    }

    /// <summary>Creates a guard with an explicit clock and state lifetime.</summary>
    public InMemoryPlatformOpenIdConnectStateReplayGuard(TimeProvider timeProvider, TimeSpan lifetime)
    {
        _timeProvider = timeProvider ?? throw new ArgumentNullException(nameof(timeProvider));
        if (lifetime <= TimeSpan.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(lifetime), lifetime, "The state lifetime must be positive.");
        }

        _lifetime = lifetime;
    }

    /// <inheritdoc />
    public Task<bool> TryConsumeAsync(string state, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(state);
        cancellationToken.ThrowIfCancellationRequested();

        var now = _timeProvider.GetUtcNow();
        PurgeExpired(now);

        var key = Hash(state);
        var accepted = _consumed.TryAdd(key, now + _lifetime);
        return Task.FromResult(accepted);
    }

    private void PurgeExpired(DateTimeOffset now)
    {
        foreach (var entry in _consumed)
        {
            if (entry.Value <= now)
            {
                _consumed.TryRemove(entry.Key, out _);
            }
        }
    }

    private static string Hash(string state)
    {
        var digest = SHA256.HashData(Encoding.UTF8.GetBytes(state));
        return Convert.ToHexString(digest);
    }
}
