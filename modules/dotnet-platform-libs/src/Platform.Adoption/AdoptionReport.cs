namespace Platform.Adoption;

/// <summary>
/// The outcome of analyzing one explicit target directory.
/// </summary>
/// <param name="TargetDirectory">Resolved absolute target directory.</param>
/// <param name="ToolVersion">Version of the diagnostic contracts.</param>
/// <param name="Results">Check results in stable check order.</param>
/// <param name="ProposedEdits">Preview-only alignment suggestions; nothing was written.</param>
public sealed record AdoptionReport(
    string TargetDirectory,
    string ToolVersion,
    IReadOnlyList<AdoptionCheckResult> Results,
    IReadOnlyList<ProposedEdit> ProposedEdits)
{
    /// <summary>Gets whether any source check failed.</summary>
    public bool HasFailures => Results.Any(result => result.Status == AdoptionStatus.Failed);

    /// <summary>Gets the number of environment-blocked checks.</summary>
    public int BlockedCount => Results.Count(result => result.Status == AdoptionStatus.EnvironmentBlocked);

    /// <summary>Gets the stable automation exit code for this report.</summary>
    public int ExitCode =>
        HasFailures ? AdoptionExitCodes.Failures
        : BlockedCount > 0 ? AdoptionExitCodes.BlockedOnly
        : AdoptionExitCodes.Success;
}
