using Platform.Realtime.Messages;

namespace Platform.Realtime.Tests;

public class RealtimeMessageTests
{
    [Fact]
    public void Create_accepts_text_payload_within_bound()
    {
        var message = RealtimeMessage.Create(100, channel: "c", payloadText: "hi");

        Assert.Equal("c", message.Channel);
        Assert.Equal("hi", message.PayloadText);
        Assert.Equal(2, message.GetPayloadByteCount());
    }

    [Fact]
    public void Create_rejects_text_payload_exceeding_bound()
    {
        Assert.Throws<ArgumentException>(() => RealtimeMessage.Create(5, payloadText: "this is too long"));
    }

    [Fact]
    public void Create_rejects_binary_payload_exceeding_bound()
    {
        Assert.Throws<ArgumentException>(() => RealtimeMessage.Create(4, payload: new byte[10]));
    }

    [Fact]
    public void Create_rejects_both_payload_forms()
    {
        Assert.Throws<ArgumentException>(() =>
            RealtimeMessage.Create(100, payloadText: "a", payload: new byte[1]));
    }
}
