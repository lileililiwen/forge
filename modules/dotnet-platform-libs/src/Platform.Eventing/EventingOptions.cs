namespace Platform.Eventing;

/// <summary>
/// Configuration for the platform eventing package. All properties
/// have safe defaults; consumers override only what they need.
/// </summary>
public sealed class EventingOptions
{
    /// <summary>
    /// The configuration section name bound by
    /// <c>Platform.Eventing.DependencyInjection.ServiceCollectionExtensions.AddPlatformEventing</c>.
    /// </summary>
    public const string SectionName = "Eventing";

    /// <summary>
    /// Gets or sets the bounded capacity of the in-process bus's
    /// <see cref="System.Threading.Channels.Channel{T}"/>. The
    /// publisher is back-pressured through
    /// <see cref="System.Threading.Channels.ChannelWriter{T}.WaitToWriteAsync"/>
    /// when the channel is full. Defaults to <c>1024</c>.
    /// </summary>
    public int InProcessBoundedCapacity { get; set; } = 1024;
}
