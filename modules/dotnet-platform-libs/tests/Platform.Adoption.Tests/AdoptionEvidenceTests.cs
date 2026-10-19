using Platform.Adoption;

namespace Platform.Adoption.Tests;

public sealed class AdoptionEvidenceTests
{
    [Fact]
    public void Unreferenced_package_is_absent()
    {
        var evidence = AdoptionEvidenceClassifier.Classify("Platform.Core", referenced: false, versionCompatible: false, hasRegistration: false, hasNativeVerification: false);
        Assert.Equal(AdoptionEvidenceLevel.Absent, evidence.Level);
        Assert.False(evidence.IsProductionReady);
    }

    [Fact]
    public void Incompatible_version_is_incompatible()
    {
        var evidence = AdoptionEvidenceClassifier.Classify("Platform.Core", referenced: true, versionCompatible: false, hasRegistration: true, hasNativeVerification: true);
        Assert.Equal(AdoptionEvidenceLevel.Incompatible, evidence.Level);
        Assert.False(evidence.IsProductionReady);
    }

    [Fact]
    public void Referenced_package_without_registration_is_configured()
    {
        var evidence = AdoptionEvidenceClassifier.Classify("Platform.Core", referenced: true, versionCompatible: true, hasRegistration: false, hasNativeVerification: false);
        Assert.Equal(AdoptionEvidenceLevel.Configured, evidence.Level);
        Assert.False(evidence.IsProductionReady);
    }

    [Fact]
    public void Package_present_without_runtime_evidence_is_unverified_never_production_ready()
    {
        var evidence = AdoptionEvidenceClassifier.Classify("Platform.Billing", referenced: true, versionCompatible: true, hasRegistration: true, hasNativeVerification: false);
        Assert.Equal(AdoptionEvidenceLevel.Unverified, evidence.Level);
        Assert.False(evidence.IsProductionReady);
    }

    [Fact]
    public void Fully_evidenced_package_is_verified()
    {
        var evidence = AdoptionEvidenceClassifier.Classify("Platform.Core", referenced: true, versionCompatible: true, hasRegistration: true, hasNativeVerification: true);
        Assert.Equal(AdoptionEvidenceLevel.Verified, evidence.Level);
        Assert.True(evidence.IsProductionReady);
    }

    [Fact]
    public void Blank_package_id_is_rejected()
    {
        Assert.Throws<ArgumentException>(() => AdoptionEvidenceClassifier.Classify(" ", referenced: true, versionCompatible: true, hasRegistration: true, hasNativeVerification: true));
    }
}
