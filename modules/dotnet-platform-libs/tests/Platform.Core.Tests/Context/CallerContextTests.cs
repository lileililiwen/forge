using Platform.Core.Context;

namespace Platform.Core.Tests.Context;

public class CallerContextTests
{
    [Fact]
    public void Anonymous_has_no_subject_and_no_tenant()
    {
        var context = CallerContext.Anonymous;

        Assert.Null(context.SubjectId);
        Assert.Null(context.TenantId);
        Assert.True(context.IsAnonymous);
        Assert.False(context.HasTenant);
    }

    [Fact]
    public void Parameterless_context_is_anonymous()
    {
        var context = new CallerContext();

        Assert.Null(context.SubjectId);
        Assert.Null(context.TenantId);
        Assert.True(context.IsAnonymous);
    }

    [Fact]
    public void Subject_only_is_not_anonymous_and_has_no_tenant()
    {
        var context = new CallerContext(SubjectId: "user-1");

        Assert.Equal("user-1", context.SubjectId);
        Assert.Null(context.TenantId);
        Assert.False(context.IsAnonymous);
        Assert.False(context.HasTenant);
    }

    [Fact]
    public void Tenant_only_is_anonymous_and_has_tenant()
    {
        var context = new CallerContext(TenantId: "tenant-1");

        Assert.Null(context.SubjectId);
        Assert.Equal("tenant-1", context.TenantId);
        Assert.True(context.IsAnonymous);
        Assert.True(context.HasTenant);
    }

    [Fact]
    public void Subject_and_tenant_both_set()
    {
        var context = new CallerContext(SubjectId: "user-1", TenantId: "tenant-1");

        Assert.Equal("user-1", context.SubjectId);
        Assert.Equal("tenant-1", context.TenantId);
        Assert.False(context.IsAnonymous);
        Assert.True(context.HasTenant);
    }

    [Fact]
    public void Whitespace_subject_is_anonymous()
    {
        var context = new CallerContext(SubjectId: "   ");

        Assert.True(context.IsAnonymous);
    }

    [Fact]
    public void Whitespace_tenant_has_no_tenant()
    {
        var context = new CallerContext(TenantId: "");

        Assert.False(context.HasTenant);
    }
}
