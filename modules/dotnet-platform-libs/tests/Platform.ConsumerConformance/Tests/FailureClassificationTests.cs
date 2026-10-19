using Platform.Billing.Contracts.Providers;

namespace Platform.ConsumerConformance.Tests;

public sealed class FailureClassificationTests
{
    [Fact]
    public void ProviderFailureClassifier_classifies_timeout_as_transient()
    {
        var failure = ProviderFailureClassifier.Classify(new TimeoutException("timed out"), "subscription.lookup");
        Assert.Equal(ProviderFailureKind.Transient, failure.Kind);
        Assert.Equal("subscription.lookup", failure.Operation);
    }

    [Fact]
    public void ProviderFailureClassifier_classifies_http_request_exception_as_transient()
    {
        var failure = ProviderFailureClassifier.Classify(new HttpRequestException("connection refused"), "checkout.create");
        Assert.Equal(ProviderFailureKind.Transient, failure.Kind);
    }

    [Fact]
    public void ProviderFailureClassifier_classifies_unauthorized_as_authentication()
    {
        var failure = ProviderFailureClassifier.Classify(new UnauthorizedAccessException("nope"), "subscription.lookup");
        Assert.Equal(ProviderFailureKind.Authentication, failure.Kind);
    }

    [Fact]
    public void ProviderFailureClassifier_classifies_format_exception_as_malformed()
    {
        var failure = ProviderFailureClassifier.Classify(new FormatException("bad json"), "webhook.parse");
        Assert.Equal(ProviderFailureKind.MalformedResponse, failure.Kind);
    }

    [Fact]
    public void ProviderFailureClassifier_classifies_argument_exception_as_configuration()
    {
        var failure = ProviderFailureClassifier.Classify(new ArgumentException("missing key"), "checkout.create");
        Assert.Equal(ProviderFailureKind.Configuration, failure.Kind);
    }

    [Fact]
    public void ProviderFailureClassifier_classifies_unknown_exception_as_permanent()
    {
        var failure = ProviderFailureClassifier.Classify(new InvalidOperationException("boom"), "subscription.lookup");
        Assert.Equal(ProviderFailureKind.Permanent, failure.Kind);
    }

    [Fact]
    public void ProviderFailure_safe_message_does_not_include_exception_text()
    {
        var failure = ProviderFailureClassifier.Classify(new InvalidOperationException("secret-token-12345"), "checkout.create");
        Assert.DoesNotContain("secret-token-12345", failure.SafeMessage);
    }
}
