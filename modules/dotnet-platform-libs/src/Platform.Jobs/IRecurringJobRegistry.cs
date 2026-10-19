namespace Platform.Jobs;

/// <summary>
/// Contract for a registry that captures recurring-job descriptors
/// produced by <see cref="RecurringJobAttribute.GetDescriptor(Type)"/>.
/// Implementations live in the consumer; the platform exposes only
/// the contract so any backing store (in-memory, database, distributed
/// cache, …) can satisfy it. The registry MUST invoke
/// <see cref="IJobTelemetry"/> on every <see cref="Register"/> call
/// when a telemetry implementation is registered.
/// </summary>
public interface IRecurringJobRegistry
{
    /// <summary>
    /// Adds the supplied <paramref name="descriptor"/> to the
    /// registry. Calling <see cref="Register"/> with a descriptor
    /// whose <see cref="RecurringJobDescriptor.Name"/> has already
    /// been registered is a no-op; the earlier descriptor is kept.
    /// </summary>
    /// <param name="descriptor">The descriptor to register.</param>
    /// <exception cref="ArgumentNullException"><paramref name="descriptor"/> is <c>null</c>.</exception>
    void Register(RecurringJobDescriptor descriptor);

    /// <summary>
    /// Gets the descriptors currently registered, in the order they
    /// were first registered.
    /// </summary>
    IReadOnlyList<RecurringJobDescriptor> Registered { get; }
}
