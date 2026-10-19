namespace Platform.Eventing;

/// <summary>
/// Marker interface for typed integration events. Consumers implement
/// concrete records (typically inheriting from
/// <see cref="IntegrationEvent"/>) that travel through the
/// <see cref="IEventBus"/>.
/// </summary>
public interface IIntegrationEvent
{
}
