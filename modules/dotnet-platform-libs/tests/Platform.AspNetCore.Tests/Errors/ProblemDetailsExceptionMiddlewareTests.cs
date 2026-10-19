using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Logging.Abstractions;
using Platform.AspNetCore.Errors;
using Platform.Core.Results;

namespace Platform.AspNetCore.Tests.Errors;

public class ProblemDetailsExceptionMiddlewareTests
{
    [Fact]
    public async Task Known_platform_failure_is_mapped_to_problem_details()
    {
        var mapper = new PlatformProblemDetailsMapper();
        var middleware = new ProblemDetailsExceptionMiddleware(
            _ => throw new PlatformProblemException(Error.Validation("name is required")),
            mapper,
            NullLogger<ProblemDetailsExceptionMiddleware>.Instance);
        var context = NewContext();

        await middleware.InvokeAsync(context);

        Assert.Equal(StatusCodes.Status400BadRequest, context.Response.StatusCode);
        Assert.Equal("application/problem+json", context.Response.ContentType);
        var body = await ReadBodyAsync(context);
        Assert.Contains("\"status\":400", body);
        Assert.Contains("\"code\":\"platform.validation\"", body);
        Assert.Contains("name is required", body);
    }

    [Fact]
    public async Task Unknown_exception_is_sanitized_and_returns_500()
    {
        var mapper = new PlatformProblemDetailsMapper();
        var middleware = new ProblemDetailsExceptionMiddleware(
            _ => throw new InvalidOperationException("internal stack-trace hint"),
            mapper,
            NullLogger<ProblemDetailsExceptionMiddleware>.Instance);
        var context = NewContext();

        await middleware.InvokeAsync(context);

        Assert.Equal(StatusCodes.Status500InternalServerError, context.Response.StatusCode);
        var body = await ReadBodyAsync(context);
        Assert.DoesNotContain("internal stack-trace hint", body);
        Assert.DoesNotContain("InvalidOperationException", body);
        Assert.Contains("\"status\":500", body);
    }

    [Fact]
    public async Task Successful_pipeline_does_not_write_problem_details()
    {
        var mapper = new PlatformProblemDetailsMapper();
        var middleware = new ProblemDetailsExceptionMiddleware(
            async ctx =>
            {
                await ctx.Response.WriteAsync("ok");
            },
            mapper,
            NullLogger<ProblemDetailsExceptionMiddleware>.Instance);
        var context = NewContext();

        await middleware.InvokeAsync(context);

        Assert.Equal(200, context.Response.StatusCode);
        Assert.Equal("ok", await ReadBodyAsync(context));
    }

    private static DefaultHttpContext NewContext()
    {
        var context = new DefaultHttpContext();
        context.Response.Body = new MemoryStream();
        return context;
    }

    private static async Task<string> ReadBodyAsync(HttpContext context)
    {
        context.Response.Body.Seek(0, SeekOrigin.Begin);
        using var reader = new StreamReader(context.Response.Body);
        return await reader.ReadToEndAsync();
    }
}
