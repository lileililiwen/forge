namespace Platform.ConsumerConformance.Fixtures.Bootstrap;

// Shared empty source for fixture consumers. The bootstrap tests only need a
// buildable .csproj; this file keeps every fixture project buildable without
// adding real production code.
internal static class BootstrapFixtureSource
{
    public static void Touch()
    {
    }
}
