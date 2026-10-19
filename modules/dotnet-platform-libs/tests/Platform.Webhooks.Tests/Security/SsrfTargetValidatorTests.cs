using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.Security;

namespace Platform.Webhooks.Tests.Security;

public sealed class SsrfTargetValidatorTests
{
    [Fact]
    public void Http_target_is_rejected()
    {
        var validator = new SsrfTargetValidator(new WebhookOptions());
        var result = validator.Validate(new Uri("http://example.invalid/hook"));
        Assert.False(result.Allowed);
        Assert.Equal(WebhookTargetRejection.InsecureScheme, result.Rejection);
    }

    [Fact]
    public void Loopback_target_is_rejected_by_default()
    {
        var validator = new SsrfTargetValidator(new WebhookOptions());
        var result = validator.Validate(new Uri("https://127.0.0.1/hook"));
        Assert.False(result.Allowed);
        Assert.Equal(WebhookTargetRejection.Loopback, result.Rejection);
    }

    [Fact]
    public void Loopback_is_allowed_when_configured()
    {
        var validator = new SsrfTargetValidator(new WebhookOptions { AllowLoopbackTargets = true });
        var result = validator.Validate(new Uri("https://127.0.0.1/hook"));
        Assert.True(result.Allowed);
    }

    [Fact]
    public void Private_network_is_rejected()
    {
        var validator = new SsrfTargetValidator(new WebhookOptions());
        var result = validator.Validate(new Uri("https://10.0.0.5/hook"));
        Assert.False(result.Allowed);
        Assert.Equal(WebhookTargetRejection.PrivateNetwork, result.Rejection);
    }

    [Fact]
    public void Link_local_is_rejected()
    {
        var validator = new SsrfTargetValidator(new WebhookOptions());
        var result = validator.Validate(new Uri("https://169.254.169.254/hook"));
        Assert.False(result.Allowed);
        Assert.Equal(WebhookTargetRejection.LinkLocal, result.Rejection);
    }

    [Fact]
    public void Relative_uri_is_rejected()
    {
        var validator = new SsrfTargetValidator(new WebhookOptions());
        var result = validator.Validate(new Uri("/hook", UriKind.Relative));
        Assert.False(result.Allowed);
        Assert.Equal(WebhookTargetRejection.NotAbsolute, result.Rejection);
    }

    [Fact]
    public void Allow_list_with_cidr_accepts_only_allowed_targets()
    {
        var validator = new SsrfTargetValidator(new WebhookOptions
        {
            TargetAllowList = new[] { "203.0.113.0/24" },
        });
        Assert.True(validator.Validate(new Uri("https://203.0.113.42/hook")).Allowed);
        Assert.False(validator.Validate(new Uri("https://198.51.100.10/hook")).Allowed);
    }
}
