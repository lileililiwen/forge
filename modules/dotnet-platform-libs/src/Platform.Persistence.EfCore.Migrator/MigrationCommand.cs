namespace Platform.Persistence.EfCore.Migrator;

/// <summary>
/// Minimal parsed console command for the migration host adapter.
/// Parsing never reads configuration or touches the database; the
/// application owns everything the runner needs through
/// <see cref="MigrationRunnerRequest"/>.
/// </summary>
/// <param name="Verb">The normalized verb (<c>apply</c> or <c>list-pending</c>), or the raw unknown verb.</param>
/// <param name="Seed">Whether <c>--seed</c> was supplied.</param>
/// <param name="Help">Whether help was requested.</param>
/// <param name="IsValid">Whether the verb is known.</param>
public sealed record MigrationCommand(string Verb, bool Seed, bool Help, bool IsValid)
{
    /// <summary>The apply verb: inspect, migrate, then optionally seed.</summary>
    public const string Apply = "apply";

    /// <summary>The list-pending verb: print pending identifiers without modifying the database.</summary>
    public const string ListPending = "list-pending";

    /// <summary>Parses console arguments into a command.</summary>
    /// <param name="args">The raw console arguments.</param>
    public static MigrationCommand Parse(string[] args)
    {
        ArgumentNullException.ThrowIfNull(args);
        var rawVerb = args.FirstOrDefault(static argument => !argument.StartsWith('-')) ?? Apply;
        var verb = string.Equals(rawVerb, Apply, StringComparison.OrdinalIgnoreCase)
            ? Apply
            : string.Equals(rawVerb, ListPending, StringComparison.OrdinalIgnoreCase)
                ? ListPending
                : rawVerb;
        var seed = args.Any(static argument => string.Equals(argument, "--seed", StringComparison.OrdinalIgnoreCase));
        var help = args.Any(static argument =>
            string.Equals(argument, "--help", StringComparison.OrdinalIgnoreCase) ||
            string.Equals(argument, "-h", StringComparison.OrdinalIgnoreCase));
        var isValid = string.Equals(verb, Apply, StringComparison.Ordinal) ||
            string.Equals(verb, ListPending, StringComparison.Ordinal);
        return new MigrationCommand(verb, seed, help, isValid);
    }

    /// <summary>Usage text printed for help and usage errors.</summary>
    public const string HelpText = """
        Platform migrator — apply application-owned EF Core migrations outside API startup.

        Usage:
          migrator [verb] [options]

        Verbs:
          apply           Apply pending migrations (default). Use --seed to also run the application seed callback.
          list-pending    Print pending migrations without applying anything.

        Options:
          --seed               After apply, also run the application seed callback.
          -h, --help           Print this help text.

        Exit codes:
          0 — success or help
          1 — migration, seed, configuration, or cancellation failure
          2 — usage error (unknown verb)
        """;
}
