namespace Platform.Web.Resilience;

/// <summary>Emits resilience events to a host-owned sink (typically the platform web telemetry logger).</summary>
public interface IHttpResilienceTelemetry
{
    /// <summary>Records a resilience decision.</summary>
    void Record(HttpResilienceEvent resilienceEvent);
}
