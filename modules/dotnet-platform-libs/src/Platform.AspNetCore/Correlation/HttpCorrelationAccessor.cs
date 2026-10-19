using Microsoft.AspNetCore.Http;
using Platform.AspNetCore.Correlation;

namespace Platform.AspNetCore.Correlation;

/// <summary>
/// <see cref="ICorrelationAccessor"/> implementation backed by the
/// current <see cref="HttpContext"/>. The accessor is registered as
/// scoped, so it resolves the correlation identifier from the request
/// the consumer is executing in.
/// </summary>
public sealed class HttpCorrelationAccessor : ICorrelationAccessor
{
    private const string ItemKey = "Platform.Correlation.Id";

    private readonly IHttpContextAccessor _httpContextAccessor;

    /// <summary>
    /// Initializes a new instance of the
    /// <see cref="HttpCorrelationAccessor"/> class.
    /// </summary>
    /// <param name="httpContextAccessor">The HTTP context accessor.</param>
    public HttpCorrelationAccessor(IHttpContextAccessor httpContextAccessor)
    {
        _httpContextAccessor = httpContextAccessor;
    }

    /// <inheritdoc />
    public string Current
    {
        get
        {
            var context = _httpContextAccessor.HttpContext;
            if (context is null)
            {
                return string.Empty;
            }

            if (context.Items.TryGetValue(HttpContextItemsKey, out var value) && value is string id)
            {
                return id;
            }

            return string.Empty;
        }
    }

    /// <summary>
    /// The <see cref="HttpContext.Items"/> key used to store the
    /// current request's correlation identifier. Exposed so the
    /// correlation middleware can use the same key without duplicating
    /// the literal.
    /// </summary>
    public static string HttpContextItemsKey => ItemKey;
}
