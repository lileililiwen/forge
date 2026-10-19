namespace Platform.Adoption;

/// <summary>
/// Deterministic classifier for per-package adoption evidence. The classifier
/// performs no I/O; callers supply the observed facts and receive a level that
/// distinguishes absent, configured, incompatible, unverified, and verified
/// usage. A package without native consumer evidence is never classified as
/// production-ready.
/// </summary>
public static class AdoptionEvidenceClassifier
{
    /// <summary>
    /// Classifies a single package reference.
    /// </summary>
    /// <param name="packageId">The platform package identifier.</param>
    /// <param name="referenced">Whether the consumer references the package.</param>
    /// <param name="versionCompatible">Whether the referenced version matches the platform baseline.</param>
    /// <param name="hasRegistration">Whether the consumer registers the package services.</param>
    /// <param name="hasNativeVerification">Whether native consumer tests or gates verify the integration.</param>
    public static AdoptionEvidence Classify(
        string packageId,
        bool referenced,
        bool versionCompatible,
        bool hasRegistration,
        bool hasNativeVerification)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(packageId);
        if (!referenced)
        {
            return new AdoptionEvidence(packageId, AdoptionEvidenceLevel.Absent, $"{packageId} is not referenced.");
        }

        if (!versionCompatible)
        {
            return new AdoptionEvidence(packageId, AdoptionEvidenceLevel.Incompatible, $"{packageId} is referenced with an incompatible version.");
        }

        if (!hasRegistration)
        {
            return new AdoptionEvidence(packageId, AdoptionEvidenceLevel.Configured, $"{packageId} is referenced but shows no registration evidence.");
        }

        if (!hasNativeVerification)
        {
            return new AdoptionEvidence(packageId, AdoptionEvidenceLevel.Unverified, $"{packageId} is registered but has no native verification evidence; it must not be described as production-ready.");
        }

        return new AdoptionEvidence(packageId, AdoptionEvidenceLevel.Verified, $"{packageId} is referenced, registered, and verified by native consumer evidence.");
    }
}
