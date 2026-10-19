namespace Platform.Persistence.EfCore.Paging;

/// <summary>Describes a bounded page request.</summary>
public sealed record PageRequest
{
    /// <summary>Creates a page request.</summary>
    public PageRequest(int pageNumber = 1, int pageSize = 25, PageSort sort = PageSort.Ascending)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(pageNumber, 1);
        ArgumentOutOfRangeException.ThrowIfLessThan(pageSize, 1);
        ArgumentOutOfRangeException.ThrowIfGreaterThan(pageSize, 500);
        PageNumber = pageNumber;
        PageSize = pageSize;
        Sort = sort;
    }

    /// <summary>Gets the one-based page number.</summary>
    public int PageNumber { get; }

    /// <summary>Gets the maximum number of records.</summary>
    public int PageSize { get; }

    /// <summary>Gets the deterministic scalar sort direction.</summary>
    public PageSort Sort { get; }
}

/// <summary>Controls the default scalar ordering used by the convenience overload.</summary>
public enum PageSort
{
    /// <summary>Sorts values from smallest to largest.</summary>
    Ascending,

    /// <summary>Sorts values from largest to smallest.</summary>
    Descending,
}

/// <summary>Contains a page and its total result count.</summary>
public sealed record PageResult<T>(IReadOnlyList<T> Items, int TotalCount, int PageNumber, int PageSize)
{
    /// <summary>Gets the total number of available pages.</summary>
    public int TotalPages => (int)Math.Ceiling(TotalCount / (double)PageSize);
}
