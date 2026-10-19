namespace Platform.FeatureManagement;

/// <summary>Resolves the application-owned context used when evaluating feature flags.</summary>
/// <remarks>Implement this in application code so the platform never owns tenant records, rollout state, or billing plans. The default resolver returns an empty context.</remarks>
public interface IFeatureContextResolver
{
    /// <summary>Resolves the current <see cref="FeatureContext"/> for feature evaluation.</summary>
    /// <param name="cancellationToken">A token that may cancel the resolution.</param>
    /// <returns>The application-provided feature evaluation context.</returns>
    ValueTask<FeatureContext> ResolveAsync(CancellationToken cancellationToken = default);
}
