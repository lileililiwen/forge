using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Options;
using Platform.AspNetCore.Correlation;

namespace Platform.AspNetCore.Tests.Correlation;

public class CorrelationMiddlewareTests
{
    [Fact]
    public async Task Generates_correlation_when_request_has_no_header()
    {
        var middleware = new CorrelationMiddleware(_ => Task.CompletedTask);
        var context = NewContext();
        var options = Options.Create(new PlatformAspNetCoreOptions());

        await middleware.InvokeAsync(context, options);

        var stored = Assert.IsType<string>(context.Items[HttpCorrelationAccessor.HttpContextItemsKey]);
        Assert.False(string.IsNullOrWhiteSpace(stored));
        Assert.NotEqual(string.Empty, stored);
    }

    [Fact]
    public async Task Generates_correlation_when_incoming_headers_not_accepted()
    {
        var middleware = new CorrelationMiddleware(_ => Task.CompletedTask);
        var context = NewContext();
        context.Request.Headers["X-Correlation-Id"] = "client-supplied";
        var options = Options.Create(new PlatformAspNetCoreOptions
        {
            AcceptIncomingCorrelationHeader = false,
        });

        await middleware.InvokeAsync(context, options);

        var stored = Assert.IsType<string>(context.Items[HttpCorrelationAccessor.HttpContextItemsKey]);
        Assert.NotEqual("client-supplied", stored);
    }

    [Fact]
    public async Task Reuses_incoming_correlation_when_opted_in()
    {
        var middleware = new CorrelationMiddleware(_ => Task.CompletedTask);
        var context = NewContext();
        context.Request.Headers["X-Correlation-Id"] = "trusted-trace";
        var options = Options.Create(new PlatformAspNetCoreOptions
        {
            AcceptIncomingCorrelationHeader = true,
        });

        await middleware.InvokeAsync(context, options);

        var stored = Assert.IsType<string>(context.Items[HttpCorrelationAccessor.HttpContextItemsKey]);
        Assert.Equal("trusted-trace", stored);
    }

    [Fact]
    public async Task Empty_incoming_header_is_ignored_when_opted_in()
    {
        var middleware = new CorrelationMiddleware(_ => Task.CompletedTask);
        var context = NewContext();
        context.Request.Headers["X-Correlation-Id"] = "   ";
        var options = Options.Create(new PlatformAspNetCoreOptions
        {
            AcceptIncomingCorrelationHeader = true,
        });

        await middleware.InvokeAsync(context, options);

        var stored = Assert.IsType<string>(context.Items[HttpCorrelationAccessor.HttpContextItemsKey]);
        Assert.False(string.IsNullOrWhiteSpace(stored));
        Assert.NotEqual("   ", stored);
    }

    private static DefaultHttpContext NewContext() => new();
}
