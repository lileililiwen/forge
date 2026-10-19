namespace Platform.Mailing.Tests;

public class MailSendResultTests
{
    [Fact]
    public void Default_constructor_records_Sent_outcome()
    {
        var result = new MailSendResult(MailSendOutcome.Sent, "msg-1");

        Assert.Equal(MailSendOutcome.Sent, result.Outcome);
        Assert.Equal("msg-1", result.ProviderMessageId);
        Assert.Null(result.ErrorCode);
        Assert.Null(result.ErrorMessage);
        Assert.True(result.IsAccepted);
    }

    [Fact]
    public void Transient_failure_is_not_accepted()
    {
        var result = new MailSendResult(
            MailSendOutcome.TransientFailure,
            ErrorCode: "throttle",
            ErrorMessage: "rate limited");

        Assert.False(result.IsAccepted);
        Assert.Equal("throttle", result.ErrorCode);
    }

    [Fact]
    public void Permanent_failure_is_not_accepted()
    {
        var result = new MailSendResult(MailSendOutcome.PermanentFailure);

        Assert.False(result.IsAccepted);
    }

    [Fact]
    public void Bounce_is_treated_as_accepted()
    {
        var result = new MailSendResult(MailSendOutcome.Bounced);

        Assert.True(result.IsAccepted);
    }
}
