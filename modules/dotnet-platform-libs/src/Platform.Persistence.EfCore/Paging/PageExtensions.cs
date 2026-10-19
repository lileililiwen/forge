using Microsoft.EntityFrameworkCore;
using System.Linq.Expressions;

namespace Platform.Persistence.EfCore.Paging;

/// <summary>Applies ownership-preserving paging to consumer queryables.</summary>
public static class PageExtensions
{
    /// <summary>Pages using a caller-provided ordering.</summary>
    public static async Task<PageResult<T>> PageAsync<T, TKey>(
        this IQueryable<T> source,
        PageRequest request,
        Expression<Func<T, TKey>> orderBy,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(source);
        ArgumentNullException.ThrowIfNull(request);
        ArgumentNullException.ThrowIfNull(orderBy);
        var total = await source.CountAsync(cancellationToken);
        var query = request.Sort == PageSort.Descending
            ? source.OrderByDescending(orderBy)
            : source.OrderBy(orderBy);
        var items = await query.Skip((request.PageNumber - 1) * request.PageSize)
            .Take(request.PageSize).ToListAsync(cancellationToken);
        return new PageResult<T>(items, total, request.PageNumber, request.PageSize);
    }

    /// <summary>Pages an in-memory or provider queryable using its scalar value as the key.</summary>
    public static PageResult<T> Page<T>(this IQueryable<T> source, PageRequest request)
    {
        ArgumentNullException.ThrowIfNull(source);
        ArgumentNullException.ThrowIfNull(request);
        var total = source.Count();
        var ordered = request.Sort == PageSort.Descending ? source.OrderByDescending(x => x) : source.OrderBy(x => x);
        var items = ordered.Skip((request.PageNumber - 1) * request.PageSize).Take(request.PageSize).ToArray();
        return new PageResult<T>(items, total, request.PageNumber, request.PageSize);
    }
}
