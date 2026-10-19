namespace Platform.Auditing.Contracts;

/// <summary>The severity of an audit event. Ordered from least to most severe.</summary>
public enum AuditSeverity
{
    /// <summary>Most verbose trace-level detail.</summary>
    Trace = 0,

    /// <summary>Developer-facing debug detail.</summary>
    Debug = 1,

    /// <summary>Normal operational information.</summary>
    Information = 2,

    /// <summary>A recoverable or noteworthy condition.</summary>
    Warning = 3,

    /// <summary>An error that did not abort the operation.</summary>
    Error = 4,

    /// <summary>A severe error that may require operator attention.</summary>
    Critical = 5,
}
