namespace Platform.Jobs.Tests;

public class BackgroundJobsOptionsTests
{
    [Fact]
    public void Default_time_zone_is_utc()
    {
        var options = new BackgroundJobsOptions();

        Assert.Equal("UTC", options.DefaultTimeZone);
    }

    [Fact]
    public void Section_name_constant_is_stable()
    {
        Assert.Equal("BackgroundJobs", BackgroundJobsOptions.SectionName);
    }
}
