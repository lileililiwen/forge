using System.Data.Common;
using System.Net.Http;
using System.Net.Sockets;

namespace Platform.Auditing.AspNetCore.Capture;

/// <summary>A safe classification of an exception for audit purposes. It never carries the exception message.</summary>
/// <param name="Kind">The safe category, for example <c>validation</c>, <c>not_found</c>, or <c>server_error</c>.</param>
public readonly record struct AuditExceptionClassification(string Kind)
{
    /// <summary>A client input problem.</summary>
    public const string Validation = "validation";

    /// <summary>A requested resource was not found.</summary>
    public const string NotFound = "not_found";

    /// <summary>A concurrency or conflict failure.</summary>
    public const string Conflict = "conflict";

    /// <summary>An upstream dependency or network failure.</summary>
    public const string Dependency = "dependency";

    /// <summary>A timeout or cancellation.</summary>
    public const string Timeout = "timeout";

    /// <summary>An unclassified server-side failure.</summary>
    public const string ServerError = "server_error";

    /// <summary>An unrecognized failure.</summary>
    public const string Unknown = "unknown";
}

/// <summary>Maps exceptions to a safe audit category without exposing the message, stack, or inner details.</summary>
public static class AuditExceptionClassifier
{
    /// <summary>Classifies <paramref name="exception"/> into a safe category.</summary>
    /// <param name="exception">The exception to classify.</param>
    /// <returns>The safe classification.</returns>
    public static AuditExceptionClassification Classify(Exception exception)
    {
        ArgumentNullException.ThrowIfNull(exception);
        switch (exception)
        {
            case ArgumentException:
            case FormatException:
            case InvalidDataException:
                return new AuditExceptionClassification(AuditExceptionClassification.Validation);
            case KeyNotFoundException:
            case FileNotFoundException:
            case DirectoryNotFoundException:
            case EntryPointNotFoundException:
                return new AuditExceptionClassification(AuditExceptionClassification.NotFound);
            case SocketException:
            case HttpRequestException:
            case DbException:
                return new AuditExceptionClassification(AuditExceptionClassification.Dependency);
            case TimeoutException:
            case TaskCanceledException:
            case OperationCanceledException:
                return new AuditExceptionClassification(AuditExceptionClassification.Timeout);
            default:
                return new AuditExceptionClassification(AuditExceptionClassification.ServerError);
        }
    }
}
