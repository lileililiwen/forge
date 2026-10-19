using Platform.Privacy.Contracts;

namespace Platform.Privacy.AspNetCore.State;

/// <summary>Request-scoped consent state, resolved from the consent cookie by middleware.</summary>
public sealed class ConsentState
{
    /// <summary>Subject the state was resolved for, or null when unresolved.</summary>
    public string? SubjectId { get; internal set; }

    /// <summary>Record the state was built from, or null when none exists or parses.</summary>
    public ConsentRecord? Record { get; internal set; }

    /// <summary>Policy version the record was evaluated against.</summary>
    public int PolicyVersion { get; internal set; }

    /// <summary>Categories granted for this request. Necessary is always present.</summary>
    public IReadOnlySet<ConsentCategory> Granted { get; internal set; } = new HashSet<ConsentCategory> { ConsentCategory.Necessary };
}
