using Platform.Admin.Contracts;

namespace Platform.Admin.Tests;

public sealed class AdminContractTests
{
    [Fact]
    public void Query_rejects_page_sizes_above_the_configured_bound()
    {
        var query = new AdminQuery(PageSize: 101);

        Assert.Throws<ArgumentOutOfRangeException>(() => query.Normalize(100, new HashSet<string>(["id"])));
    }

    [Fact]
    public void Query_rejects_unknown_sort_keys_and_normalizes_search()
    {
        var query = new AdminQuery(Search: "  alice  ", SortBy: "email");

        Assert.Throws<ArgumentException>(() => query.Normalize(100, new HashSet<string>(["id"])));
        Assert.Equal("alice", (query with { SortBy = "id" }).Normalize(100, new HashSet<string>(["id"])).Search);
    }
}
