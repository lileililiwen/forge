namespace Platform.Auditing.Contracts;

/// <summary>The normalized outcome of the action an audit event describes.</summary>
public enum AuditOutcome
{
    /// <summary>The action completed successfully.</summary>
    Success = 0,

    /// <summary>The action failed.</summary>
    Failure = 1,

    /// <summary>The action was denied (for example, an authorization check).</summary>
    Denied = 2,

    /// <summary>An exception occurred while performing the action.</summary>
    Error = 3,

    /// <summary>The outcome could not be determined.</summary>
    Unknown = 4,
}
