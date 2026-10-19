namespace Platform.FeatureManagement;

/// <summary>Default <see cref="IFeatureContextResolver"/> that returns an empty context.</summary>
/// <remarks>Applications replace this with a resolver that supplies tenant and subject context from their own ambient state (for example the multitenancy accessor).</remarks>
public sealed class DefaultFeatureContextResolver : IFeatureContextResolver
{
    /// <inheritdoc/>
    public ValueTask<FeatureContext> ResolveAsync(CancellationToken cancellationToken = default) =>
        new(new FeatureContext());
}
