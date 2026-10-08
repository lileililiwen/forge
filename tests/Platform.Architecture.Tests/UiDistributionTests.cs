using System.Security.Cryptography;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Xml.Linq;

namespace Platform.Architecture.Tests;

/// <summary>
/// Guards for the UI token distribution change (openspec change
/// ui-shared-layer-reachability). Additive methods in an existing project: no new
/// .csproj, no new package, no public API change, and no existing guard touched.
///
/// The subject of most of these is the <c>ui/</c> npm layer and the frozen
/// baseline data file, not .NET code. They exist because a check that no gate
/// runs does not gate (design.md D18) — <c>forge/scripts/contract-parity.sh</c> is
/// this workspace's diagnosed example of a check that reports OK without checking.
/// </summary>
public sealed class UiDistributionTests
{
    private const int RecordedLegacyDivergentCount = 15;

    /// <summary>
    /// sha256 of the generated CSS after this change's two additive semantic
    /// color tokens (<c>--color-on-primary</c>, <c>--color-border</c>) were added by
    /// platform-shared-auth-pages. Any change to the emitted bytes is a token contract
    /// change and must be reviewed here, not discovered by a consumer whose
    /// <c>file:</c> digest no longer matches.
    /// </summary>
    private const string GeneratedTokensCssSha256 =
        "2216411d7e42ef5399c79c0b84e34460cff97251aa1fa47a09870f455be790f3";

    private static readonly string RepositoryRoot = LocateRepositoryRoot();

    private static string Path(string relative) => System.IO.Path.Combine(RepositoryRoot, relative);

    [Fact]
    public void Frozen_legacy_divergent_list_has_not_grown()
    {
        var baseline = ReadLegacyBaseline();
        var entries = baseline.GetProperty("divergentPrimitive").EnumerateArray().ToList();

        Assert.Equal(RecordedLegacyDivergentCount, entries.Count);
        Assert.Equal(
            RecordedLegacyDivergentCount,
            baseline.GetProperty("divergentPrimitiveCount").GetInt32());
        Assert.Equal(
            entries.Count,
            entries.Select(entry => entry.GetProperty("repo").GetString()).Distinct(StringComparer.Ordinal).Count());

        // D17: arivio migrates first because its copy is a measured a11y defect
        // (it omits padding, margin and border, so the element leaks a 1px box),
        // not a style disagreement like the other fourteen.
        var first = entries.Single(entry => entry.GetProperty("repo").GetString() == "arivio");
        Assert.Equal(1, first.GetProperty("migrationOrder").GetInt32());
        Assert.Equal("a11y-defect", first.GetProperty("severity").GetString());
        Assert.Equal(
            entries.Count,
            entries.Select(entry => entry.GetProperty("migrationOrder").GetInt32()).Distinct().Count());
    }

    [Fact]
    public void A_sixteenth_divergent_repository_fails_the_frozen_list_guard()
    {
        // The guard's own predicate, exercised on a synthetic sixteenth entry. There is
        // no API, flag or argument anywhere that adds one: the list is a data file that
        // no code path writes, which is why D16 chose a file over a script constant.
        var recorded = ReadLegacyBaseline()
            .GetProperty("divergentPrimitive")
            .EnumerateArray()
            .Select(entry => entry.GetProperty("repo").GetString()!)
            .ToList();
        recorded.Add("sixteenth-repo");

        var error = Assert.Throws<InvalidOperationException>(() => AssertFrozenListHasNotGrown(recorded));
        Assert.Contains("ui-legacy-list-grew", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void No_script_in_the_repository_writes_the_frozen_baseline()
    {
        // D16: the list is a data file so no code path can extend it. A writer would
        // have to name the file on the right of a redirect, in a writeFile, or in an
        // in-place edit; none of those exist.
        var offenders = new List<string>();
        foreach (var file in Directory.EnumerateFiles(RepositoryRoot, "*", SearchOption.AllDirectories))
        {
            if (IsBuildOutput(file))
            {
                continue;
            }

            var relative = System.IO.Path.GetRelativePath(RepositoryRoot, file);
            if (relative.StartsWith(".git", StringComparison.Ordinal))
            {
                continue;
            }

            if (!file.EndsWith(".sh", StringComparison.Ordinal) && !file.EndsWith(".mjs", StringComparison.Ordinal))
            {
                continue;
            }

            var text = File.ReadAllText(file);
            foreach (var line in text.Split('\n'))
            {
                if (!line.Contains("legacy-divergent.json", StringComparison.Ordinal))
                {
                    continue;
                }

                var isWrite = Regex.IsMatch(line, @">\s*[^>]*legacy-divergent\.json")
                    || line.Contains("writeFile", StringComparison.Ordinal)
                    || Regex.IsMatch(line, @"\bsed\s+-i\b")
                    || line.Contains("'w'", StringComparison.Ordinal);
                if (isWrite)
                {
                    offenders.Add($"{relative}: {line.Trim()}");
                }
            }
        }

        Assert.Empty(offenders);
    }

    [Fact]
    public void Generated_token_css_is_byte_stable()
    {
        Assert.Equal(GeneratedTokensCssSha256, Sha256(Path("ui/packages/design-tokens/dist/tokens.css")));
    }

    [Fact]
    public void Generated_token_css_defines_the_canonical_visually_hidden_primitive()
    {
        var css = File.ReadAllText(Path("ui/packages/design-tokens/dist/tokens.css"));
        var match = Regex.Match(css, @"(?<![-\w])\.visually-hidden\s*\{(?<body>[^}]*)\}");
        Assert.True(match.Success, "tokens.css must define the canonical .visually-hidden primitive");

        var body = match.Groups["body"].Value;
        foreach (var declaration in new[]
                 {
                     "position: absolute", "width: 1px", "height: 1px", "padding: 0", "margin: -1px",
                     "overflow: hidden", "clip: rect(0 0 0 0)", "white-space: nowrap", "border: 0",
                 })
        {
            Assert.Contains(declaration, body, StringComparison.Ordinal);
        }

        Assert.DoesNotContain("!important", body, StringComparison.Ordinal);
        Assert.Equal(9, Regex.Matches(body, "[a-z-]+\\s*:").Count);
    }

    [Fact]
    public void The_canonical_primitive_is_defined_in_exactly_one_generated_artifact()
    {
        var artifacts = new[]
        {
            "ui/packages/design-tokens/dist/tokens.css",
            "ui/packages/react-ui/styles.css",
            "src/Platform.UI.Razor/wwwroot/platform-tokens.css",
            "src/Platform.UI.Razor/wwwroot/platform-ui.css",
        };

        var defined = artifacts
            .Where(relative => Regex.IsMatch(
                File.ReadAllText(Path(relative)),
                @"\.(visually-hidden|sr-only)(?![\w-])[^{};]*\{"))
            .ToList();

        Assert.Equal(new[] { "ui/packages/design-tokens/dist/tokens.css" }, defined);
    }

    [Fact]
    public void Committed_feed_tarballs_match_the_feed_manifest_digests()
    {
        var feed = JsonDocument.Parse(File.ReadAllText(Path("ui/feed/feed.json"))).RootElement;
        Assert.Equal("0.1.0", feed.GetProperty("kitVersion").GetString());

        foreach (var package in feed.GetProperty("packages").EnumerateArray())
        {
            var name = package.GetProperty("name").GetString()!;
            var file = package.GetProperty("file").GetString()!;
            Assert.Equal("0.1.0", package.GetProperty("version").GetString());
            Assert.Equal(package.GetProperty("sha256").GetString(), Sha256(Path($"ui/feed/{file}")));
            Assert.Contains("package/package.json", package.GetProperty("members").EnumerateArray().Select(m => m.GetString()));
            Assert.StartsWith("@platform/", name, StringComparison.Ordinal);
        }
    }

    [Fact]
    public void Ui_npm_packages_stay_at_0_1_0_with_digest_pinning()
    {
        // D15: the change is additive (a .visually-hidden primitive, dark-theme
        // tokens, and the two semantic colors platform-shared-auth-pages measured its
        // way to), so a 0.2.0 bump would signal a contract change that did not happen.
        // The digest, not the version, is the load-bearing check.
        foreach (var package in new[] { "design-tokens", "react-ui", "react-shell", "react-auth", "react-templates" })
        {
            var manifest = JsonDocument.Parse(
                File.ReadAllText(Path($"ui/packages/{package}/package.json"))).RootElement;
            Assert.Equal("0.1.0", manifest.GetProperty("version").GetString());
        }

        Assert.Contains(
            "sha256",
            File.ReadAllText(Path("ui/scripts/verify-ui-feed.mjs")),
            StringComparison.Ordinal);
    }

    [Fact]
    public void React_templates_use_platform_tokens_and_do_not_duplicate_base_primitives()
    {
        var package = JsonDocument.Parse(
            File.ReadAllText(Path("ui/packages/react-templates/package.json"))).RootElement;
        Assert.Equal("@platform/react-templates", package.GetProperty("name").GetString());
        Assert.Equal("./src/index.ts", package.GetProperty("exports").GetProperty(".").GetString());
        Assert.Equal("./styles.css", package.GetProperty("exports").GetProperty("./styles.css").GetString());
        Assert.Equal(">=18", package.GetProperty("peerDependencies").GetProperty("react").GetString());
        Assert.Equal(">=18", package.GetProperty("peerDependencies").GetProperty("@types/react").GetString());
        Assert.Equal(">=5 <7", package.GetProperty("peerDependencies").GetProperty("antd").GetString());

        var layouts = File.ReadAllText(Path("ui/packages/react-templates/src/layouts.tsx"));
        foreach (var layout in new[] { "PublicPageLayout", "UserDashboardLayout", "AdminDashboardLayout", "DataTable" })
        {
            Assert.Contains($"function {layout}", layouts, StringComparison.Ordinal);
        }
        Assert.Contains("headingRef.current?.focus()", layouts, StringComparison.Ordinal);
        Assert.Contains("kind=\"forbidden\"", layouts, StringComparison.Ordinal);
        foreach (var primitive in new[] { "Button", "Field", "Card", "Badge", "State" })
        {
            Assert.DoesNotMatch($"export\\s+(?:function|const)\\s+{primitive}\\b", layouts);
        }

        var theme = File.ReadAllText(Path("ui/packages/react-templates/src/theme.tsx"));
        Assert.Contains("semanticColorValues", theme, StringComparison.Ordinal);
        Assert.Contains("tokenValues", theme, StringComparison.Ordinal);
        Assert.False(Regex.IsMatch(theme, "#(?:[0-9a-f]{3,8})\\b|\\brgb\\(", RegexOptions.IgnoreCase));

        var styles = File.ReadAllText(Path("ui/packages/react-templates/styles.css"));
        Assert.Contains("@media (max-width: 48rem)", styles, StringComparison.Ordinal);
        Assert.Contains("@media (max-width: 30rem)", styles, StringComparison.Ordinal);
        Assert.Contains(".platform-data-table-region > .ant-table-wrapper", styles, StringComparison.Ordinal);
        Assert.Contains("minmax(0, 1fr)", styles, StringComparison.Ordinal);
        Assert.DoesNotContain("@import", styles, StringComparison.Ordinal);
        Assert.False(Regex.IsMatch(styles, "#[0-9a-f]{3,8}\\b|\\brgb\\(", RegexOptions.IgnoreCase));
    }

    [Fact]
    public void No_dotnet_project_references_a_ui_npm_package()
    {
        // The .NET and npm layers stay decoupled: a .NET project may not gain a
        // dependency on an npm package, and the conformance check is a shell script
        // precisely because its subject is repositories' text files (design.md D7).
        var projects = Directory.EnumerateFiles(Path("src"), "*.csproj", SearchOption.AllDirectories)
            .Concat(Directory.EnumerateFiles(Path("tests"), "*.csproj", SearchOption.AllDirectories))
            .Concat(Directory.EnumerateFiles(Path("samples"), "*.csproj", SearchOption.AllDirectories))
            .Concat(Directory.EnumerateFiles(Path("templates"), "*.csproj", SearchOption.AllDirectories))
            .ToList();
        Assert.NotEmpty(projects);

        foreach (var project in projects)
        {
            var text = File.ReadAllText(project);
            Assert.DoesNotContain("@platform/design-tokens", text, StringComparison.Ordinal);
            Assert.DoesNotContain("@platform/react-ui", text, StringComparison.Ordinal);
            Assert.DoesNotContain("@platform/react-shell", text, StringComparison.Ordinal);
        }
    }

    [Fact]
    public void The_public_api_coverage_gap_for_platform_ui_razor_stays_recorded_not_closed()
    {
        // D14: scripts/check-public-api.sh scans six packages and does not cover
        // Platform.UI.Razor, and eng/public-api-baseline.txt is deliberately not
        // regenerated. A regenerated baseline could silently absorb a breaking change
        // in a package this change does not touch, so the gap is recorded instead of
        // closed. If this test ever fails, the gap was closed on purpose: regenerate
        // the baseline in that change and update the ruling with its evidence.
        var scan = File.ReadAllText(Path("scripts/check-public-api.sh"));
        Assert.DoesNotContain("Platform.UI.Razor", scan, StringComparison.Ordinal);
        Assert.True(File.Exists(Path("eng/public-api-baseline.txt")));

        // The gap is also named in the change's own documentation, so it stays visible.
        var design = File.ReadAllText(Path("openspec/changes/archive/2026-10-05-ui-shared-layer-reachability/design.md"));
        Assert.Contains("D14", design, StringComparison.Ordinal);
        Assert.Contains("in that scan list", design, StringComparison.Ordinal);
    }

    [Fact]
    public void The_ui_verifier_is_wired_into_the_repository_quality_gate()
    {
        // D18: the repo gate is dotnet + openspec + git diff --check. Without the
        // wiring below, a token regression does not block it - the contract-parity
        // failure class, reproduced.
        var gate = File.ReadAllText(Path("scripts/quality-gate.sh"));
        Assert.Contains("node ui/scripts/verify-ui.mjs", gate, StringComparison.Ordinal);
        Assert.Contains("ui/scripts/selftest-verify-ui.mjs", gate, StringComparison.Ordinal);
    }

    [Fact]
    public void The_package_manifest_and_razor_package_version_are_untouched_by_this_capability()
    {
        // The UI capability adds no .csproj, PackageReference or ProjectReference
        // and does not change Platform.UI.Razor. The count is a tripwire for that
        // claim, not a statement about the size of the platform, so it moves when
        // a different capability adds a package. platform-build-policy-package
        // added src/Platform.Build (a build-only policy package, no assembly and
        // no references), and later capabilities took the manifest to 78; the
        // assertions below are what actually pin the UI capability's blast radius.
        var manifest = JsonDocument.Parse(File.ReadAllText(Path("eng/package-manifest.json"))).RootElement;
        Assert.Equal(78, manifest.GetProperty("packages").GetArrayLength());

        var razor = manifest.GetProperty("packages").EnumerateArray()
            .Single(package => package.GetProperty("packageId").GetString() == "Platform.UI.Razor");
        Assert.Equal("0.1.0", razor.GetProperty("version").GetString());
        Assert.Empty(razor.GetProperty("packageReferences").EnumerateArray());
        Assert.Empty(razor.GetProperty("projectReferences").EnumerateArray());
    }

    private static void AssertFrozenListHasNotGrown(IReadOnlyCollection<string> repos)
    {
        if (repos.Count != RecordedLegacyDivergentCount)
        {
            throw new InvalidOperationException(
                $"ui-legacy-list-grew: the frozen legacy-divergent baseline holds {repos.Count} entries, " +
                $"recorded at {RecordedLegacyDivergentCount}. The list is closed by construction: a repository " +
                "migrates off it, it is never extended, and there is no suppression flag that adds an entry.");
        }
    }

    private static JsonElement ReadLegacyBaseline()
    {
        var frozen = JsonDocument.Parse(File.ReadAllText(Path("ui/legacy-divergent.json"))).RootElement;
        Assert.True(frozen.GetProperty("frozen").GetBoolean());
        return frozen;
    }

    private static bool IsBuildOutput(string file)
    {
        var relative = System.IO.Path.GetRelativePath(RepositoryRoot, file);
        return relative.Contains($"{System.IO.Path.DirectorySeparatorChar}bin{System.IO.Path.DirectorySeparatorChar}", StringComparison.Ordinal)
            || relative.Contains($"{System.IO.Path.DirectorySeparatorChar}obj{System.IO.Path.DirectorySeparatorChar}", StringComparison.Ordinal)
            || relative.Contains($"{System.IO.Path.DirectorySeparatorChar}node_modules{System.IO.Path.DirectorySeparatorChar}", StringComparison.Ordinal);
    }

    private static string Sha256(string path)
    {
        using var stream = File.OpenRead(path);
        return Convert.ToHexString(SHA256.HashData(stream)).ToLowerInvariant();
    }

    private static string LocateRepositoryRoot()
    {
        var current = new DirectoryInfo(AppContext.BaseDirectory);
        while (current is not null)
        {
            if (File.Exists(System.IO.Path.Combine(current.FullName, "Directory.Build.props"))
                && File.Exists(System.IO.Path.Combine(current.FullName, "Platform.sln")))
            {
                return current.FullName;
            }

            current = current.Parent;
        }

        throw new InvalidOperationException(
            "Unable to locate the repository root from " + AppContext.BaseDirectory);
    }
}
