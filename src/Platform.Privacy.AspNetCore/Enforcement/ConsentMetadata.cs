using Microsoft.AspNetCore.Http;
using Platform.Privacy.Contracts;

namespace Platform.Privacy.AspNetCore.Enforcement;

/// <summary>Maps the request to a subject identifier, or null when unresolved.</summary>
public interface IConsentSubjectResolver
{
    /// <summary>Resolves the subject, or null when the request carries no usable identity.</summary>
    string? Resolve(HttpContext context);
}

/// <summary>Policy for requests with no resolved subject.</summary>
public enum UnresolvedSubjectBehavior
{
    /// <summary>Fail closed: optional categories denied.</summary>
    Deny = 0,

    /// <summary>Serve Necessary-only without a subject record.</summary>
    NecessaryOnly = 1,
}

/// <summary>Declares consent categories an endpoint requires.</summary>
[AttributeUsage(AttributeTargets.Class | AttributeTargets.Method, AllowMultiple = false)]
public sealed class RequireConsentAttribute : Attribute
{
    /// <summary>Creates the metadata.</summary>
    public RequireConsentAttribute(params ConsentCategory[] categories)
    {
        ArgumentNullException.ThrowIfNull(categories);
        if (categories.Length == 0)
        {
            throw new ArgumentException("At least one category is required.", nameof(categories));
        }

        Categories = categories.Distinct().ToArray();
    }

    /// <summary>Required categories.</summary>
    public IReadOnlyList<ConsentCategory> Categories { get; }
}
