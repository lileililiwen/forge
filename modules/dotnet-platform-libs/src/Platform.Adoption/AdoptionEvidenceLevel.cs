namespace Platform.Adoption;

/// <summary>
/// Evidence level for a single platform package in a consumer repository.
/// The level never claims production readiness without native project evidence.
/// </summary>
public enum AdoptionEvidenceLevel
{
    /// <summary>The package is not referenced by the consumer.</summary>
    Absent,

    /// <summary>
    /// The package is referenced with a compatible version but the consumer
    /// shows no registration or verification evidence yet.
    /// </summary>
    Configured,

    /// <summary>The referenced version is incompatible with the platform baseline.</summary>
    Incompatible,

    /// <summary>
    /// The package is referenced and registered but the consumer has no native
    /// verification evidence (tests, gates) for the integration.
    /// </summary>
    Unverified,

    /// <summary>
    /// The package is referenced, registered, and backed by native consumer
    /// verification evidence.
    /// </summary>
    Verified,
}
