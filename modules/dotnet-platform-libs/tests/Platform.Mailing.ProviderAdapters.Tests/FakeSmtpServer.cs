using System.Net;
using System.Net.Sockets;
using System.Text;

namespace Platform.Mailing.ProviderAdapters.Tests;

/// <summary>
/// Deterministic in-process SMTP server used to exercise the SMTP adapter
/// over a real loopback socket. The server speaks the minimal RFC 5321
/// command sequence the adapter needs and records every conversation so
/// tests can assert on the constructed MIME payload.
/// </summary>
internal sealed class FakeSmtpServer : IDisposable
{
    private const string AuthPrefix = "AUTH PLAIN";

    private readonly TcpListener _listener;
    private readonly CancellationTokenSource _stopping = new();

    public FakeSmtpServer()
    {
        _listener = new TcpListener(IPAddress.Loopback, 0);
        _listener.Start();
        _ = Task.Run(AcceptLoopAsync);
    }

    public int Port => ((IPEndPoint)_listener.LocalEndpoint).Port;

    public List<Conversation> Conversations { get; } = new();

    /// <summary>Gets or sets the reply code returned to MAIL FROM commands.</summary>
    public int MailReplyCode { get; set; } = 250;

    /// <summary>Gets or sets the reply code returned to RCPT TO commands.</summary>
    public int RcptReplyCode { get; set; } = 250;

    /// <summary>Gets or sets a value indicating whether the EHLO response advertises AUTH PLAIN.</summary>
    public bool AdvertiseAuth { get; set; }

    /// <summary>Gets or sets the delay before the greeting, used to exercise the operation timeout.</summary>
    public TimeSpan GreetingDelay { get; set; }

    public sealed class Conversation
    {
        public List<string> Commands { get; } = new();

        public List<string> DataLines { get; } = new();

        public string? AuthPayload { get; set; }

        public string Data => string.Join("\r\n", DataLines);
    }

    public void Dispose()
    {
        _stopping.Cancel();
        _listener.Stop();
        _stopping.Dispose();
    }

    private async Task AcceptLoopAsync()
    {
        try
        {
            while (!_stopping.IsCancellationRequested)
            {
                var client = await _listener.AcceptTcpClientAsync(_stopping.Token);
                _ = Task.Run(() => HandleClientAsync(client));
            }
        }
        catch (OperationCanceledException)
        {
        }
        catch (ObjectDisposedException)
        {
        }
        catch (SocketException)
        {
        }
    }

    private async Task HandleClientAsync(TcpClient client)
    {
        using var tcp = client;
        var conversation = new Conversation();
        lock (Conversations)
        {
            Conversations.Add(conversation);
        }

        var stream = tcp.GetStream();
        using var reader = new StreamReader(stream, Encoding.ASCII);
        try
        {
            if (GreetingDelay > TimeSpan.Zero)
            {
                await Task.Delay(GreetingDelay);
            }

            await WriteAsync(stream, "220 fake ESMTP ready\r\n");
            while (await reader.ReadLineAsync() is { } line)
            {
                conversation.Commands.Add(line);
                var upper = line.ToUpperInvariant();
                if (upper.StartsWith("EHLO", StringComparison.Ordinal) || upper.StartsWith("HELO", StringComparison.Ordinal))
                {
                    var extensions = AdvertiseAuth
                        ? "250-fake.local\r\n250-8BITMIME\r\n250-AUTH PLAIN\r\n250 SIZE 10485760\r\n"
                        : "250-fake.local\r\n250-8BITMIME\r\n250 SIZE 10485760\r\n";
                    await WriteAsync(stream, extensions);
                }
                else if (upper.StartsWith(AuthPrefix, StringComparison.Ordinal))
                {
                    var payload = line.Length > AuthPrefix.Length + 1
                        ? line[(AuthPrefix.Length + 1)..].Trim()
                        : string.Empty;
                    conversation.AuthPayload = payload.Length == 0
                        ? null
                        : Encoding.UTF8.GetString(Convert.FromBase64String(payload));
                    await WriteAsync(stream, "235 accepted\r\n");
                }
                else if (upper.StartsWith("MAIL FROM", StringComparison.Ordinal))
                {
                    await WriteAsync(stream, $"{MailReplyCode} reply\r\n");
                    if (MailReplyCode >= 400)
                    {
                        break;
                    }
                }
                else if (upper.StartsWith("RCPT TO", StringComparison.Ordinal))
                {
                    await WriteAsync(stream, $"{RcptReplyCode} reply\r\n");
                }
                else if (upper.StartsWith("DATA", StringComparison.Ordinal))
                {
                    await WriteAsync(stream, "354 end data with <CR><LF>.<CR><LF>\r\n");
                    while (await reader.ReadLineAsync() is { } dataLine)
                    {
                        if (dataLine == ".")
                        {
                            break;
                        }

                        conversation.DataLines.Add(dataLine);
                    }

                    await WriteAsync(stream, "250 accepted\r\n");
                }
                else if (upper == "QUIT")
                {
                    await WriteAsync(stream, "221 bye\r\n");
                    break;
                }
                else
                {
                    await WriteAsync(stream, "250 ok\r\n");
                }
            }
        }
        catch (IOException)
        {
        }
        catch (ObjectDisposedException)
        {
        }
        catch (SocketException)
        {
        }
        catch (OperationCanceledException)
        {
        }
    }

    private static async Task WriteAsync(NetworkStream stream, string payload)
    {
        var bytes = Encoding.ASCII.GetBytes(payload);
        await stream.WriteAsync(bytes);
        await stream.FlushAsync();
    }
}
