namespace Platform.Adoption;

/// <summary>Per-package adoption evidence reported for a consumer repository.</summary>
/// <param name="PackageId">The platform package identifier.</param>
/// <param name="Level">The evidence level; never production-ready without native evidence.</param>
/// <param name="Detail">Human-readable detail supporting the level.</param>
public sealed record AdoptionEvidence(string PackageId, AdoptionEvidenceLevel Level, string Detail)
{
    /// <summary>Gets whether the package may be described as production-ready.</summary>
    public bool IsProductionReady => Level == AdoptionEvidenceLevel.Verified;
}
