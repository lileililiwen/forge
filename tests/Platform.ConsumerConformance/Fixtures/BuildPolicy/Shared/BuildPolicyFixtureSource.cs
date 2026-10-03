namespace Platform.ConsumerConformance.Fixtures.BuildPolicy;

// Shared empty source for the build-policy fixture consumers. The tests only
// need a buildable .csproj; this file keeps every fixture project buildable
// without adding real production code.
internal static class BuildPolicyFixtureSource
{
    public static void Touch()
    {
    }
}
