namespace Platform.Core.Results;

#pragma warning disable CA1716 // Intentional: "Error" is the conventional name for failure descriptions in this style of API.

/// <summary>
/// A framework-neutral failure description. Carries a stable
/// <see cref="Code"/> that higher layers can translate, a safe
/// <see cref="Message"/> suitable for surfacing to callers, and optional
/// structured <see cref="Metadata"/> for diagnostics.
/// </summary>
/// <param name="Code">A stable, machine-readable identifier for the failure.</param>
/// <param name="Message">A human-readable, caller-safe description of the failure.</param>
/// <param name="Metadata">Optional structured metadata for diagnostics.</param>
public sealed record Error(
    string Code,
    string Message,
    IReadOnlyDictionary<string, object?>? Metadata = null)
{
    /// <summary>
    /// Creates a validation-style error using the
    /// <c>platform.validation</c> code family.
    /// </summary>
    /// <param name="message">The safe validation message.</param>
    /// <returns>A new <see cref="Error"/> with code <c>platform.validation</c>.</returns>
    public static Error Validation(string message) =>
        new(ValidationCode, message);

    /// <summary>
    /// Creates a not-found error using the <c>platform.not_found</c>
    /// code family.
    /// </summary>
    /// <param name="message">The safe not-found message.</param>
    /// <returns>A new <see cref="Error"/> with code <c>platform.not_found</c>.</returns>
    public static Error NotFound(string message) =>
        new(NotFoundCode, message);

    /// <summary>Stable code used for validation failures.</summary>
    public const string ValidationCode = "platform.validation";

    /// <summary>Stable code used when a referenced entity is missing.</summary>
    public const string NotFoundCode = "platform.not_found";
}

#pragma warning restore CA1716
