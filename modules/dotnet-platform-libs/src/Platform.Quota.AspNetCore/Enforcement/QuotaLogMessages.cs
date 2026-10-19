using Microsoft.Extensions.Logging;

namespace Platform.Quota.AspNetCore.Enforcement;

/// <summary>Strongly-typed quota log messages.</summary>
internal static class QuotaLogMessages
{
    private static readonly Action<ILogger, Exception> SubjectResolutionFailed =
        LoggerMessage.Define(LogLevel.Warning, new EventId(1, "QuotaSubjectResolutionFailed"), "Quota subject resolution failed; applying missing-context policy.");

    private static readonly Action<ILogger, Exception> StoreUnavailable =
        LoggerMessage.Define(LogLevel.Error, new EventId(2, "QuotaStoreUnavailable"), "Quota store became unavailable; applying provider-unavailable policy.");

    private static readonly Action<ILogger, string, string, Exception> SettleFailed =
        LoggerMessage.Define<string, string>(LogLevel.Warning, new EventId(3, "QuotaSettleFailed"), "Failed to settle quota reservation {OperationKey} for {Subject}.");

    private static readonly Action<ILogger, string, string, Exception> ReleaseFailed =
        LoggerMessage.Define<string, string>(LogLevel.Warning, new EventId(4, "QuotaReleaseFailed"), "Failed to release quota reservation {OperationKey} for {Subject}.");

    /// <summary>Logs a subject resolution failure.</summary>
    public static void LogSubjectResolutionFailed(ILogger logger, Exception exception) => SubjectResolutionFailed(logger, exception);

    /// <summary>Logs a store unavailability.</summary>
    public static void LogStoreUnavailable(ILogger logger, Exception exception) => StoreUnavailable(logger, exception);

    /// <summary>Logs a settlement failure.</summary>
    public static void LogSettleFailed(ILogger logger, string operationKey, string subject, Exception exception) => SettleFailed(logger, operationKey, subject, exception);

    /// <summary>Logs a release failure.</summary>
    public static void LogReleaseFailed(ILogger logger, string operationKey, string subject, Exception exception) => ReleaseFailed(logger, operationKey, subject, exception);
}
