using Platform.Privacy.Anonymization;
using Platform.Privacy.Contracts;
using Platform.Privacy.Evaluation;
using Platform.Privacy.Stores;

namespace Platform.Privacy.Tests;

public class ConsentPolicyTests
{
    private static ConsentRecord Record(int version, params ConsentCategory[] granted) =>
        new("sub1", granted.ToHashSet(), DateTimeOffset.UtcNow, version);

    [Fact]
    public void Necessary_is_granted_without_a_record()
    {
        Assert.True(new ConsentPolicy(1).IsGranted(ConsentCategory.Necessary, null));
    }

    [Fact]
    public void Optional_category_is_denied_without_a_record()
    {
        Assert.False(new ConsentPolicy(1).IsGranted(ConsentCategory.Analytics, null));
    }

    [Fact]
    public void Granted_category_under_current_version_is_granted()
    {
        Assert.True(new ConsentPolicy(2).IsGranted(ConsentCategory.Analytics, Record(2, ConsentCategory.Analytics)));
    }

    [Fact]
    public void Stale_version_voids_optional_grants()
    {
        Assert.False(new ConsentPolicy(3).IsGranted(ConsentCategory.Analytics, Record(2, ConsentCategory.Analytics)));
    }

    [Fact]
    public async Task Store_round_trips_a_record()
    {
        var store = new InMemoryConsentStore();
        var record = Record(1, ConsentCategory.Preferences);
        await store.SetAsync(record);
        Assert.Equal(record, await store.GetAsync("sub1"));
        Assert.Null(await store.GetAsync("unknown"));
    }
}

public class SubjectRightsRegistryTests
{
    private sealed class OkExport(string name) : ISubjectExportProvider
    {
        public string Name => name;
        public Task<SubjectExport> ExportAsync(string subjectId, CancellationToken cancellationToken = default) =>
            Task.FromResult(new SubjectExport(name, new Dictionary<string, string> { ["profile"] = subjectId }));
    }

    private sealed class BadExport(string name) : ISubjectExportProvider
    {
        public string Name => name;
        public Task<SubjectExport> ExportAsync(string subjectId, CancellationToken cancellationToken = default) =>
            throw new InvalidOperationException("boom");
    }

    private sealed class OkErasure(string name, long count) : ISubjectErasureProvider
    {
        public string Name => name;
        public Task<ProviderErasure> EraseAsync(string subjectId, CancellationToken cancellationToken = default) =>
            Task.FromResult(new ProviderErasure(name, count));
    }

    [Fact]
    public async Task Export_keeps_successes_and_records_failure()
    {
        var registry = new SubjectRightsRegistry().AddExport(new OkExport("a")).AddExport(new BadExport("b"));
        var bundle = await registry.ExportAsync("sub1");
        Assert.Single(bundle.Exports);
        Assert.Equal("sub1", bundle.Exports[0].Documents["profile"]);
        Assert.True(bundle.Failures.ContainsKey("b"));
    }

    [Fact]
    public async Task Erasure_aggregates_counts_unchanged()
    {
        var registry = new SubjectRightsRegistry().AddErasure(new OkErasure("a", 3)).AddErasure(new OkErasure("b", 7));
        var report = await registry.EraseAsync("sub1");
        Assert.Equal(10, report.Providers.Sum(p => p.ErasedItems));
    }
}

public class AnonymizerTests
{
    [Fact]
    public void MaskEmail_keeps_shape()
    {
        Assert.Equal("j**e@example.com", Anonymizers.MaskEmail("jane@example.com"));
    }

    [Fact]
    public void Pseudonym_is_stable_and_keyed()
    {
        var key = new byte[] { 1, 2, 3 };
        Assert.Equal(Anonymizers.StablePseudonym("a@b.c", key), Anonymizers.StablePseudonym("a@b.c", key));
        Assert.NotEqual(
            Anonymizers.StablePseudonym("a@b.c", key),
            Anonymizers.StablePseudonym("a@b.c", new byte[] { 4, 5, 6 }));
    }
}
