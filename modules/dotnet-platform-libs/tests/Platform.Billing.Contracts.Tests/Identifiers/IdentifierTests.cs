using Platform.Billing.Contracts.Identifiers;

namespace Platform.Billing.Contracts.Tests.Identifiers;

public class IdentifierTests
{
    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("   ")]
    public void PlanId_create_rejects_invalid_values(string? value)
    {
        Assert.Throws<ArgumentException>(() => PlanId.Create(value!));
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("   ")]
    public void FeatureKey_create_rejects_invalid_values(string? value)
    {
        Assert.Throws<ArgumentException>(() => FeatureKey.Create(value!));
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("   ")]
    public void SubjectKey_create_rejects_invalid_values(string? value)
    {
        Assert.Throws<ArgumentException>(() => SubjectKey.Create(value!));
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("   ")]
    public void ProviderName_create_rejects_invalid_values(string? value)
    {
        Assert.Throws<ArgumentException>(() => ProviderName.Create(value!));
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("   ")]
    public void ProviderEventId_create_rejects_invalid_values(string? value)
    {
        Assert.Throws<ArgumentException>(() => ProviderEventId.Create(value!));
    }

    [Fact]
    public void SubjectKey_anonymous_marks_IsAnonymous()
    {
        Assert.True(SubjectKey.Anonymous.IsAnonymous);
    }

    [Fact]
    public void Non_anonymous_subject_is_not_anonymous()
    {
        var subject = SubjectKey.Create("user-1");
        Assert.False(subject.IsAnonymous);
    }
}
