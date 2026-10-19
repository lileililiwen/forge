using Platform.Jobs.Hangfire;

namespace Platform.Jobs.Hangfire.Tests;

public class HangfireJobsOptionsTests : HangfireTest
{
    [Fact]
    public void Defaults_are_safe_and_bounded()
    {
        var options = new HangfireJobsOptions();

        Assert.Equal(HangfireStorageKind.InMemory, options.Storage);
        Assert.Null(options.PostgreSqlConnectionString);
        Assert.Equal("default", options.Queue);
        Assert.Equal(new[] { "default" }, options.Queues);
        Assert.Equal(5, options.WorkerCount);
        Assert.Equal(TimeSpan.FromSeconds(30), options.SchedulePollingInterval);
        Assert.Equal(TimeSpan.FromSeconds(30), options.HeartbeatInterval);
        Assert.False(options.DashboardEnabled);
        Assert.Equal("/jobs", options.DashboardRoute);
        Assert.Null(options.DashboardAuthorization);
        options.Validate();
    }

    [Fact]
    public void Section_name_is_stable()
    {
        Assert.Equal("BackgroundJobs:Hangfire", HangfireJobsOptions.SectionName);
    }

    [Fact]
    public void PostgreSql_storage_requires_a_connection_string()
    {
        var options = new HangfireJobsOptions { Storage = HangfireStorageKind.PostgreSql };

        var exception = Assert.Throws<ArgumentException>(options.Validate);
        Assert.DoesNotContain(":", exception.Message);
    }

    [Fact]
    public void Oversized_connection_string_is_rejected()
    {
        var options = new HangfireJobsOptions
        {
            Storage = HangfireStorageKind.PostgreSql,
            PostgreSqlConnectionString = new string('a', 4097),
        };

        Assert.Throws<ArgumentException>(options.Validate);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(101)]
    [InlineData(-1)]
    public void Worker_count_must_be_between_1_and_100(int workerCount)
    {
        var options = new HangfireJobsOptions { WorkerCount = workerCount };

        Assert.Throws<ArgumentOutOfRangeException>(options.Validate);
    }

    [Fact]
    public void Empty_queue_list_is_rejected()
    {
        var options = new HangfireJobsOptions { Queues = Array.Empty<string>() };

        Assert.Throws<ArgumentException>(options.Validate);
    }

    [Fact]
    public void Whitespace_queue_names_are_rejected()
    {
        var options = new HangfireJobsOptions { Queue = "default queue" };

        Assert.Throws<ArgumentException>(options.Validate);
    }

    [Fact]
    public void Intervals_must_be_bounded()
    {
        var options = new HangfireJobsOptions { SchedulePollingInterval = TimeSpan.FromMilliseconds(50) };
        Assert.Throws<ArgumentOutOfRangeException>(options.Validate);

        var options2 = new HangfireJobsOptions { HeartbeatInterval = TimeSpan.FromMinutes(11) };
        Assert.Throws<ArgumentOutOfRangeException>(options2.Validate);
    }

    [Fact]
    public void Dashboard_route_must_start_with_a_slash()
    {
        var options = new HangfireJobsOptions { DashboardRoute = "jobs" };

        Assert.Throws<ArgumentException>(options.Validate);
    }

    [Fact]
    public void Enabled_dashboard_requires_an_authorization_callback()
    {
        var options = new HangfireJobsOptions { DashboardEnabled = true };

        Assert.Throws<ArgumentException>(options.Validate);
    }

    [Fact]
    public void Enabled_dashboard_with_callback_is_valid()
    {
        var options = new HangfireJobsOptions
        {
            DashboardEnabled = true,
            DashboardAuthorization = _ => true,
        };

        options.Validate();
    }
}
