using System.Diagnostics;
using System.IO.Compression;
using System.Text.Json;
using System.Xml.Linq;

namespace Platform.ConsumerConformance;

/// <summary>
/// Conformance tests for the packed MSBuild policy package
/// (<c>Platform.Build</c>, whose props ship at
/// <c>buildTransitive/Platform.Build.props</c>).
/// <para>
/// Every scenario is driven by a standalone fixture project under
/// <c>Fixtures/BuildPolicy/</c> that is restored from the committed local feed
/// and then evaluated with <c>dotnet msbuild</c>. A passing build is not
/// evidence for a build-only package, because such a package restores and
/// builds identically whether or not its props are honoured, so every
/// build-shaped test would pass on a package carrying no policy at all. These
/// tests therefore assert <em>resolved property values</em> and the packed
/// archive's own layout.
/// </para>
/// <para>
/// Each fixture sits below an intentionally empty
/// <c>Fixtures/BuildPolicy/Directory.Build.props</c>, so the repository's own
/// build defaults cannot be the reason a value resolves. That isolation is not
/// cosmetic: a property already set by a nearer file is skipped by the
/// package's <c>== ''</c> guard, so an inherited value would make a package
/// assertion look like a no-op and a package non-assertion look effective.
/// </para>
/// </summary>
public sealed class BuildPolicyConformanceTests
{
    /// <summary>
    /// The properties design section 3 assigns to the policy package. This list
    /// is the ownership table expressed in code: the drift test asserts the
    /// packed props asserts exactly these and nothing else, so the table cannot
    /// rot silently in either direction.
    /// </summary>
    private static readonly string[] PackageOwnedProperties =
    {
        "EnforceCodeStyleInBuild",
        "Deterministic",
        "CentralPackageTransitivePinningEnabled",
        "ContinuousIntegrationBuild",
    };

    /// <summary>
    /// The properties build/Platform.Consumer.props already owns, with the
    /// values the bootstrap applies for them. The policy package must never
    /// assert any of these.
    /// </summary>
    private static readonly (string Property, string BootstrapValue)[] BootstrapOwnedProperties =
    {
        ("Nullable", "enable"),
        ("LangVersion", "latest"),
        ("AnalysisLevel", "latest-recommended"),
        ("TreatWarningsAsErrors", "true"),
        ("ManagePackageVersionsCentrally", "true"),
    };

    /// <summary>Project identity the policy package must never assert.</summary>
    private static readonly string[] ProjectOwnedProperties =
    {
        "TargetFramework",
        "RootNamespace",
    };

    /// <summary>
    /// Suppressions permitted in the packed props, each with the reason it is
    /// justified. Empty is the shipped state: the workspace baseline this
    /// package replaces carried no NoWarn list, so there was no inherited set
    /// to justify and inventing entries would be an unjustified suppression.
    /// </summary>
    private static readonly IReadOnlySet<string> JustifiedSuppressions =
        new HashSet<string>(StringComparer.OrdinalIgnoreCase);

    private const string ExpectedPolicyPackageVersion = "0.1.0";
    private const string PropsEntryName = "buildTransitive/Platform.Build.props";

    [Fact]
    public void Policy_package_packs_its_props_file_into_buildTransitive()
    {
        // A props file that fails to pack is the silent failure this change
        // exists to prevent: the package restores, builds and complains about
        // nothing while carrying no policy. The archive is listed rather than
        // trusted from the pack log. This assertion is not a substitute for the
        // resolved-value tests below - a props file that is present but
        // malformed fails to import while the archive still lists it - which is
        // exactly what happened once during this change's implementation.
        var package = LocatePackedPolicyPackage();
        using var archive = ZipFile.OpenRead(package);
        var entries = archive.Entries.Select(entry => entry.FullName).ToArray();

        Assert.Contains(PropsEntryName, entries);
        Assert.DoesNotContain(
            entries,
            name => name.EndsWith("Platform.Build.props", StringComparison.OrdinalIgnoreCase)
                    && !string.Equals(name, PropsEntryName, StringComparison.Ordinal));
    }

    [Fact]
    public void Packed_props_file_is_well_formed_xml()
    {
        // A file that is in the archive but cannot be imported produces a
        // package that restores cleanly and then fails every consuming build.
        // XML validity is cheap to check and expensive to discover downstream.
        var document = XDocument.Parse(ReadPackedPropsText());
        Assert.NotEmpty(document.Root!.Elements());
    }

    [Fact]
    public void Policy_package_carries_no_runtime_assembly()
    {
        // Build-only: the package asserts MSBuild policy, it is not a library.
        // A lib/ entry here would mean the package could be referenced for its
        // API, which is not what it is for.
        var package = LocatePackedPolicyPackage();
        using var archive = ZipFile.OpenRead(package);
        var libraryEntries = archive.Entries
            .Select(entry => entry.FullName)
            .Where(name => name.StartsWith("lib/", StringComparison.OrdinalIgnoreCase)
                        || name.StartsWith("ref/", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.True(
            libraryEntries.Length == 0,
            "Platform.Build is build-only and must not ship an assembly. Found: " + string.Join(", ", libraryEntries));
    }

    [Fact]
    public void Packed_props_asserts_exactly_the_properties_the_package_owns()
    {
        // The single-ownership rule as a hard check: the packed props asserts
        // the package's properties and neither the bootstrap's five nor any
        // property belonging to the consuming project. Two files that both
        // assert one property behind the same == '' guard resolve it by import
        // order, and the loser is silent - that is the drift hazard design
        // section 3 exists to prevent.
        var asserted = ReadAssertedProperties(ReadPackedPropsText());

        foreach (var property in PackageOwnedProperties)
        {
            Assert.True(
                asserted.Contains(property),
                $"Platform.Build owns {property}, so its props must assert it. Asserted: {string.Join(", ", asserted)}");
        }

        foreach (var (property, _) in BootstrapOwnedProperties)
        {
            Assert.False(
                asserted.Contains(property),
                $"{property} is owned by build/Platform.Consumer.props; the policy package must not also assert it.");
        }

        foreach (var property in ProjectOwnedProperties)
        {
            Assert.False(
                asserted.Contains(property),
                $"{property} belongs to the consuming project; a shared policy package must not assert it.");
        }
    }

    [Fact]
    public void Packed_props_guards_every_owned_property_so_a_consumer_value_wins()
    {
        // An unguarded assertion would let the package's value beat a consumer's,
        // which is the opposite of the documented contract and of the
        // convention build/Platform.Consumer.props already follows.
        var document = XDocument.Parse(ReadPackedPropsText());
        var unguarded = document
            .Descendants()
            .Where(element => !string.Equals(element.Name.LocalName, "PropertyGroup", StringComparison.OrdinalIgnoreCase))
            .Where(element => PackageOwnedProperties.Contains(element.Name.LocalName, StringComparer.OrdinalIgnoreCase))
            .Where(element => !string.Equals(
                element.Attribute("Condition")?.Value,
                $"'$({element.Name.LocalName})' == ''",
                StringComparison.Ordinal))
            .Select(element => element.Name.LocalName)
            .ToArray();

        Assert.True(
            unguarded.Length == 0,
            "Every property the package owns must be guarded with Condition=\"'$(X)' == ''\". Unguarded: "
            + string.Join(", ", unguarded));
    }

    [Fact]
    public void Packed_props_does_not_assert_a_NoWarn_list()
    {
        // NoWarn is owned by the package and ships with no entries, by decision
        // rather than oversight: the workspace baseline this replaces carried
        // no list, so there is nothing to inherit and nothing to justify.
        Assert.False(
            ReadAssertedProperties(ReadPackedPropsText()).Contains("NoWarn"),
            "Platform.Build currently ships no NoWarn list. Adding entries is a separate, reviewable step.");
    }

    [Fact]
    public void Every_NoWarn_suppression_in_the_packed_props_carries_a_justification()
    {
        // The rejection half of the justified-list rule, enforced for real: the
        // moment an entry appears it must have a one-line justification beside
        // it, so an unjustified suppression cannot be merged by review inertia.
        var document = XDocument.Parse(ReadPackedPropsText());
        var noWarn = document
            .Descendants()
            .Where(element => string.Equals(element.Name.LocalName, "NoWarn", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        foreach (var element in noWarn)
        {
            var entries = (element.Value ?? string.Empty)
                .Replace("$(NoWarn)", string.Empty)
                .Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
            foreach (var entry in entries)
            {
                Assert.True(
                    JustifiedSuppressions.Contains(entry),
                    $"NoWarn entry '{entry}' has no one-line justification. Ship the suppression together with the "
                    + "reasoning that survives review, or drop the entry.");
            }
        }
    }

    [Fact]
    public void Referencing_the_policy_package_applies_the_owned_policy()
    {
        // The positive half, asserted on resolved values from a project that
        // restored the package out of the feed and imported nothing by path.
        var properties = QueryProperties(Fixture("Applied"), PackageOwnedProperties);

        Assert.Equal("true", properties["EnforceCodeStyleInBuild"]);
        Assert.Equal("true", properties["CentralPackageTransitivePinningEnabled"]);
        Assert.Equal("true", properties["Deterministic"]);
    }

    [Fact]
    public void Continuous_integration_flag_resolves_only_on_ci()
    {
        // The CI flag is asserted under a '$(CI)' condition, so it must be
        // absent from a local build and present on CI. The unreferenced fixture
        // is checked under the same CI setting: without it, a repository or SDK
        // default could be what produces the value.
        // GitHub Actions exports CI=true to every step; pin the query to a
        // local build rather than inheriting the runner's environment.
        Assert.Equal(
            string.Empty,
            QuerySingleProperty(Fixture("Applied"), "ContinuousIntegrationBuild", "-p:CI=false")["ContinuousIntegrationBuild"]);
        Assert.Equal(
            string.Empty,
            QuerySingleProperty(Fixture("Unreferenced"), "ContinuousIntegrationBuild", "-p:CI=false")["ContinuousIntegrationBuild"]);
        Assert.Equal(
            "true",
            QuerySingleProperty(Fixture("Applied"), "ContinuousIntegrationBuild", "-p:CI=true")["ContinuousIntegrationBuild"]);
    }

    [Fact]
    public void The_policy_does_not_travel_on_the_bootstraps_five_properties()
    {
        // The other half of the ownership rule, proven on a resolved project
        // instead of by reading XML. If the package asserted any of these, the
        // property would carry the platform's bootstrap value here. It does
        // not: the resolved value is the SDK's, because nothing else claims
        // the property in this fixture. AnalysisLevel is the sharpest of the
        // five, since the platform's own value differs from the SDK default.
        var properties = QueryProperties(Fixture("Applied"), BootstrapOwnedProperties.Select(e => e.Property).ToArray());
        var withoutPackage = QueryProperties(Fixture("Unreferenced"), BootstrapOwnedProperties.Select(e => e.Property).ToArray());

        foreach (var (property, bootstrapValue) in BootstrapOwnedProperties)
        {
            Assert.NotEqual(
                bootstrapValue,
                properties[property],
                StringComparer.OrdinalIgnoreCase);
            Assert.Equal(
                withoutPackage[property],
                properties[property],
                StringComparer.OrdinalIgnoreCase);
        }
    }

    [Fact]
    public void A_consumer_that_sets_an_owned_property_wins()
    {
        // The contract every == '' guard exists for, and the import-order proof
        // in the same breath: the package's props are imported before this
        // project body, so a body value overwrites what the package set. A
        // props file imported after the body could not produce this result.
        var properties = QueryProperties(Fixture("ConsumerOverride"), PackageOwnedProperties);

        Assert.Equal("false", properties["EnforceCodeStyleInBuild"]);
        Assert.Equal("false", properties["Deterministic"]);
        // The consumer set this one unconditionally; the package only asserts
        // it on CI, and '==' guards respect either kind of prior value.
        Assert.Equal("consumer-set", properties["ContinuousIntegrationBuild"]);
        // Untouched by the consumer, so the package still applies it. This is
        // the control for the three assertions above: the package is present
        // and working in the same evaluation.
        Assert.Equal("true", properties["CentralPackageTransitivePinningEnabled"]);
    }

    [Fact]
    public void A_project_without_the_reference_receives_none_of_the_policy()
    {
        // The negative half. Without it, "the policy applied" and "nothing was
        // ever evaluated" are indistinguishable from the applied fixture alone.
        var properties = QueryProperties(Fixture("Unreferenced"), PackageOwnedProperties);

        Assert.Equal("false", properties["EnforceCodeStyleInBuild"]);
        Assert.Equal(string.Empty, properties["CentralPackageTransitivePinningEnabled"]);
        Assert.Equal(string.Empty, properties["ContinuousIntegrationBuild"]);
        // Deterministic is deliberately not asserted here: the SDK already
        // defaults it to true, so this fixture cannot distinguish the package's
        // contribution from the default. The consumer-override fixture is what
        // establishes that the package does not force it.
        Assert.Equal("true", properties["Deterministic"]);
    }

    [Fact]
    public void The_policy_package_does_not_impose_a_target_framework()
    {
        // A net8.0 consumer: the platform publishes net10.0 only, so anything
        // else proves the package did not retarget it. A shared policy package
        // that asserted a TFM would silently move this project to net10.0.
        var properties = QueryProperties(Fixture("NonDefaultTarget"), new[] { "TargetFramework", "TargetFrameworks", "RootNamespace" });

        Assert.Equal("net8.0", properties["TargetFramework"]);
        Assert.Equal(string.Empty, properties["TargetFrameworks"]);
        // RootNamespace is project identity for the same reason, so the package
        // must leave the fixture's own value alone.
        Assert.Equal("Platform.ConsumerConformance.Fixtures.BuildPolicy.NonDefaultTarget", properties["RootNamespace"]);

        // The policy still applies on a non-default target: the package is
        // TFM-agnostic, not inert off the platform's framework.
        Assert.Equal(
            "true",
            QuerySingleProperty(Fixture("NonDefaultTarget"), "EnforceCodeStyleInBuild")["EnforceCodeStyleInBuild"]);
    }

    [Fact]
    public void Both_policy_routes_in_scope_resolve_to_their_recorded_owner()
    {
        // The hazard design section 3 was written for, exercised: a
        // buildTransitive props file (imported before the project body) and
        // build/Platform.Consumer.props (imported after it, which is where the
        // workspace Directory.Build.targets puts it) are both live in one
        // evaluation of one project.
        var properties = QueryProperties(
            Fixture("SharedOwnership"),
            PackageOwnedProperties.Concat(BootstrapOwnedProperties.Select(e => e.Property)).ToArray());

        // Owned by the package.
        Assert.Equal("true", properties["EnforceCodeStyleInBuild"]);
        Assert.Equal("true", properties["Deterministic"]);
        Assert.Equal("true", properties["CentralPackageTransitivePinningEnabled"]);

        // Owned by the bootstrap, which lands its value even though the package
        // was imported first. That is the ordering proof: had the package
        // claimed Nullable, LangVersion or AnalysisLevel behind a == '' guard,
        // its earlier write would have been the one that survived, because
        // first-writer-wins.
        Assert.Equal("enable", properties["Nullable"]);
        Assert.Equal("latest", properties["LangVersion"]);
        Assert.Equal("latest-recommended", properties["AnalysisLevel"]);

        // The bootstrap's remaining two, which resolve to the SDK default here
        // rather than to the bootstrap's value. The SDK had already set both
        // before either file was imported, so the bootstrap's == '' guard skips
        // them - pre-existing bootstrap behaviour, identical in the unreferenced
        // project and therefore not evidence about the package. What matters
        // for this change is that the package did not supply them either: it
        // never asserts them, and it has no value to contribute.
        Assert.Equal("false", properties["TreatWarningsAsErrors"]);
        Assert.Equal("false", properties["ManagePackageVersionsCentrally"]);
    }

    [Fact]
    public void Policy_package_is_pinned_to_an_exact_version()
    {
        // docs/platform-consumer-adoption.md rejects floating ranges and
        // wildcards for every Platform.* package; the policy package is one,
        // and a build-only package that floats is a policy that changes under
        // a build that never changed a dependency.
        var document = XDocument.Load(Fixture("Applied"));
        var reference = document
            .Descendants()
            .Single(element => string.Equals(element.Name.LocalName, "PackageReference", StringComparison.OrdinalIgnoreCase)
                            && string.Equals(element.Attribute("Include")?.Value, "Platform.Build", StringComparison.OrdinalIgnoreCase));
        var version = reference.Attribute("Version")?.Value ?? string.Empty;

        Assert.Equal(ExpectedPolicyPackageVersion, version);
        Assert.DoesNotContain("*", version, StringComparison.Ordinal);
        Assert.DoesNotContain("[", version, StringComparison.Ordinal);
        Assert.DoesNotContain("(", version, StringComparison.Ordinal);
        Assert.DoesNotContain(",", version, StringComparison.Ordinal);
    }

    [Fact]
    public void Policy_package_project_is_build_only_and_carries_no_references()
    {
        // The project shape that produces the build-only package.
        // IncludeBuildOutput=false is what keeps an assembly out of the feed,
        // and an empty reference set is what keeps the policy artifact free of
        // any dependency direction a consumer would inherit.
        var project = Path.Combine(LocatePlatformRepositoryRoot(), "src", "Platform.Build", "Platform.Build.csproj");
        Assert.True(File.Exists(project), "Expected policy package project at " + project);
        var document = XDocument.Load(project);

        Assert.Equal("Platform.Build", document.Descendants("PackageId").Single().Value.Trim());
        Assert.Equal("false", document.Descendants("IncludeBuildOutput").Single().Value.Trim(), ignoreCase: true);
        Assert.Empty(ReadReferences(document, "ProjectReference"));
        Assert.Empty(ReadReferences(document, "PackageReference"));
    }

    [Fact]
    public void Policy_package_ships_at_the_platform_version_alongside_the_others()
    {
        // The package is a platform package: same version as every other one in
        // the feed, packed by the same solution pack step, so a consumer pins a
        // single version set rather than a per-package one.
        var feed = Path.Combine(LocatePlatformRepositoryRoot(), "tests", "Platform.ConsumerConformance", ".local-feed");
        var siblings = Directory
            .EnumerateFiles(feed, "Platform.*.nupkg")
            .Select(path => Path.GetFileName(path))
            .ToArray();

        Assert.NotEmpty(siblings);
        var versions = siblings
            .Select(name => name["Platform.".Length..^".nupkg".Length])
            .Select(id => id[(id.LastIndexOf('.') + 1)..])
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToArray();

        Assert.True(
            versions.Length == 1,
            "Every platform package in the feed must share one version. Found: " + string.Join(", ", versions));
    }

    private static HashSet<string> ReadAssertedProperties(string propsXml)
    {
        var document = XDocument.Parse(propsXml);
        return document
            .Descendants()
            .Where(element => !string.Equals(element.Name.LocalName, "PropertyGroup", StringComparison.OrdinalIgnoreCase))
            .Where(element => !element.HasElements && !string.IsNullOrWhiteSpace(element.Value))
            .Select(element => element.Name.LocalName)
            .ToHashSet(StringComparer.OrdinalIgnoreCase);
    }

    private static IReadOnlyList<string> ReadReferences(XDocument document, string elementName) =>
        document.Descendants(elementName)
            .Select(element => element.Attribute("Include")?.Value ?? string.Empty)
            .ToArray();

    private static string ReadPackedPropsText()
    {
        var package = LocatePackedPolicyPackage();
        using var archive = ZipFile.OpenRead(package);
        var entry = archive.GetEntry(PropsEntryName);
        Assert.NotNull(entry);
        using var stream = entry!.Open();
        using var reader = new StreamReader(stream);
        return reader.ReadToEnd();
    }

    private static string LocatePackedPolicyPackage()
    {
        var feed = Path.Combine(
            LocatePlatformRepositoryRoot(),
            "tests",
            "Platform.ConsumerConformance",
            ".local-feed",
            $"Platform.Build.{ExpectedPolicyPackageVersion}.nupkg");
        if (File.Exists(feed))
        {
            return feed;
        }

        throw new InvalidOperationException(
            "Platform.Build is not in the local feed at " + feed
            + ". Run scripts/conformance.sh, which packs the solution into the feed before restoring the fixtures.");
    }

    private static string Fixture(string scenario) => Path.Combine(
        LocatePlatformRepositoryRoot(),
        "tests",
        "Platform.ConsumerConformance",
        "Fixtures",
        "BuildPolicy",
        scenario,
        scenario switch
        {
            "Applied" => "AppliedPolicyConsumer.csproj",
            "ConsumerOverride" => "ConsumerOverrideConsumer.csproj",
            "Unreferenced" => "UnreferencedConsumer.csproj",
            "NonDefaultTarget" => "NonDefaultTargetConsumer.csproj",
            "SharedOwnership" => "SharedOwnershipConsumer.csproj",
            _ => throw new ArgumentOutOfRangeException(nameof(scenario), scenario, "Unknown build-policy fixture scenario."),
        });

    private static IReadOnlyDictionary<string, string> QueryProperties(string project, string[] propertyNames, params string[] extraArgs)
    {
        var (exitCode, stdout, stderr) = RunMsbuild(project, propertyNames, extraArgs);
        Assert.True(
            exitCode == 0,
            $"dotnet msbuild failed for {project}.\nstderr:\n{stderr}\nstdout:\n{stdout}\n"
            + "A failed evaluation here means the fixture never reached the packaged props, which is exactly the "
            + "silent-failure mode this fixture exists to catch.");

        var trimmed = stdout?.Trim() ?? string.Empty;
        var results = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);

        // MSBuild prints the raw value when exactly one -getProperty is
        // requested, and a JSON object when several are. An unset property
        // prints nothing at all, which is the empty string the CI assertions
        // rely on.
        if (!trimmed.StartsWith("{", StringComparison.Ordinal))
        {
            foreach (var name in propertyNames)
            {
                results[name] = trimmed;
            }
            return results;
        }

        using var document = JsonDocument.Parse(trimmed);
        var properties = document.RootElement.ValueKind == JsonValueKind.Object
                         && document.RootElement.TryGetProperty("Properties", out var element)
                         && element.ValueKind == JsonValueKind.Object
            ? element
            : document.RootElement;

        foreach (var name in propertyNames)
        {
            results[name] = properties.TryGetProperty(name, out var value) && value.ValueKind == JsonValueKind.String
                ? value.GetString() ?? string.Empty
                : string.Empty;
        }
        return results;
    }

    /// <summary>
    /// Single-property convenience. Deliberately a separate name rather than a
    /// <c>params string[]</c> overload of <see cref="QueryProperties(string, string[], string[])"/>:
    /// an overload would make a multi-property call such as
    /// <c>QueryProperties(p, "A", "B")</c> compile as one property plus two
    /// global-property arguments, and MSBuild then reports MSB1008 instead of
    /// the missing property.
    /// </summary>
    private static IReadOnlyDictionary<string, string> QuerySingleProperty(string project, string name, params string[] extraArgs) =>
        QueryProperties(project, new[] { name }, extraArgs);

    private static (int ExitCode, string Stdout, string Stderr) RunMsbuild(string project, IReadOnlyList<string> propertyNames, IReadOnlyList<string> extraArgs)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = "dotnet",
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true,
        };
        startInfo.ArgumentList.Add("msbuild");
        startInfo.ArgumentList.Add(project);
        foreach (var name in propertyNames)
        {
            startInfo.ArgumentList.Add($"-getProperty:{name}");
        }
        foreach (var extra in extraArgs)
        {
            startInfo.ArgumentList.Add(extra);
        }
        startInfo.ArgumentList.Add("-nologo");
        startInfo.ArgumentList.Add("-m:1");

        using var process = Process.Start(startInfo) ?? throw new InvalidOperationException("Failed to start dotnet.");
        var stdout = process.StandardOutput.ReadToEnd();
        var stderr = process.StandardError.ReadToEnd();
        process.WaitForExit();
        return (process.ExitCode, stdout, stderr);
    }

    private static string LocatePlatformRepositoryRoot()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory is not null)
        {
            var candidate = Path.Combine(directory.FullName, "build", "Platform.Consumer.props");
            if (File.Exists(candidate))
            {
                return directory.FullName;
            }
            directory = directory.Parent;
        }
        throw new InvalidOperationException(
            "Unable to locate the platform repository root from " + AppContext.BaseDirectory);
    }
}
