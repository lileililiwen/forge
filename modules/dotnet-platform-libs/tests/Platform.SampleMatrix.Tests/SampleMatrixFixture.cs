namespace Platform.SampleMatrix.Tests;

/// <summary>Shared repository-root resolution for matrix tests.</summary>
internal static class SampleMatrixFixture
{
    public static string RepositoryRoot()
    {
        var current = new DirectoryInfo(AppContext.BaseDirectory);
        while (current is not null)
        {
            if (File.Exists(Path.Combine(current.FullName, "Platform.sln")))
            {
                return current.FullName;
            }

            current = current.Parent;
        }

        throw new Xunit.Sdk.XunitException("Could not locate the repository root.");
    }
}
