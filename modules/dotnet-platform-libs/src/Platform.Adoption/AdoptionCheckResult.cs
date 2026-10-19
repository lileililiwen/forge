namespace Platform.Adoption;

/// <summary>
/// A single adoption diagnostic result. Evidence is secret-free: relative
/// paths, identifiers, and versions only, never file contents, credentials,
/// or exception internals.
/// </summary>
/// <param name="CheckId">Stable machine-readable check identifier.</param>
/// <param name="DisplayName">Short human-readable check name.</param>
/// <param name="Status">Result classification.</param>
/// <param name="Evidence">Bounded evidence summary supporting the classification.</param>
/// <param name="Remediation">Fix guidance, or a rerun instruction for blocked checks.</param>
public sealed record AdoptionCheckResult(
    string CheckId,
    string DisplayName,
    AdoptionStatus Status,
    string Evidence,
    string Remediation);
