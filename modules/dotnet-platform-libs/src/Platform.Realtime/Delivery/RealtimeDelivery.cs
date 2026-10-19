namespace Platform.Realtime.Delivery;

/// <summary>
/// Documents and exposes the non-durable delivery semantics of the realtime
/// transports. The platform makes no guarantee that a message sent while a
/// client was disconnected will ever be received; applications must own replay
/// or resynchronization.
/// </summary>
public static class RealtimeDelivery
{
    /// <summary>
    /// The documented delivery guarantee. Hosts surface this constant in their
    /// migration and onboarding docs. Consumers MUST NOT assume missed messages
    /// are redelivered.
    /// </summary>
    public const string Guarantee = "non-durable";

    /// <summary>
    /// Returns whether the platform guarantees durable delivery for the realtime
    /// transports. Always <c>false</c>; durability is the application's
    /// responsibility.
    /// </summary>
    public static bool IsDurable => false;
}
