using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Mailing.DependencyInjection;

namespace Platform.Mailing.Tests;

public class TestServerIntegrationTests
{
    [Fact]
    public async Task Host_with_AddPlatformMailing_resolves_clock_and_default_options()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();

        var response = await client.GetAsync("/probe");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"defaultFromAddress\":\"noreply@example.invalid\"", body);
        Assert.Contains("\"maxAttempts\":3", body);
    }

    [Fact]
    public async Task Host_with_configured_section_overrides_defaults()
    {
        await using var app = BuildApp(extraConfiguration: new Dictionary<string, string?>
        {
            ["Mailing:DefaultFromAddress"] = "team@example.com",
            ["Mailing:MaxAttempts"] = "7",
        });
        var client = app.GetTestClient();

        var response = await client.GetAsync("/probe");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"defaultFromAddress\":\"team@example.com\"", body);
        Assert.Contains("\"maxAttempts\":7", body);
    }

    [Fact]
    public async Task Host_consumer_mail_service_is_invoked_through_di()
    {
        var mailService = new RecordingMailService();

        await using var app = BuildApp(extraMailService: mailService);
        var client = app.GetTestClient();

        var response = await client.GetAsync("/send");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"outcome\":\"Sent\"", body);
        Assert.Contains("\"providerMessageId\":\"msg-1\"", body);
        var sent = Assert.Single(mailService.Sent);
        Assert.Equal("hello", sent.Subject);
    }

    private static WebApplication BuildApp(
        IReadOnlyDictionary<string, string?>? extraConfiguration = null,
        IMailService? extraMailService = null)
    {
        var options = new WebApplicationOptions
        {
            EnvironmentName = "Testing",
        };
        var builder = WebApplication.CreateBuilder(options);
        builder.WebHost.UseTestServer();
        if (extraConfiguration is not null)
        {
            builder.Configuration.AddInMemoryCollection(extraConfiguration);
        }

        builder.Services.AddPlatformMailing(mailingOptions =>
            builder.Configuration
                .GetSection(MailingOptions.SectionName)
                .Bind(mailingOptions));

        if (extraMailService is not null)
        {
            builder.Services.AddSingleton(extraMailService);
        }

        var app = builder.Build();
        app.MapGet("/probe", (IOptions<MailingOptions> options, IClock clock) => new
        {
            defaultFromAddress = options.Value.DefaultFromAddress,
            maxAttempts = options.Value.MaxAttempts,
            clock = clock.UtcNow.ToString("O"),
        });
        app.MapGet("/send", async ([FromServices] IMailService service) =>
        {
            var result = await service.SendAsync(new MailMessage(
                MailAddress.Create("from@example.com"),
                new[] { MailAddress.Create("to@example.com") },
                "hello",
                textBody: "body"));
            return Results.Ok(new
            {
                outcome = result.Outcome.ToString(),
                providerMessageId = result.ProviderMessageId,
            });
        });

        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }
}
