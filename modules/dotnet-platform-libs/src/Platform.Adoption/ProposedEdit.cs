namespace Platform.Adoption;

/// <summary>
/// A proposed package alignment edit. Preview-only: the analyzer reports
/// these without writing any file.
/// </summary>
/// <param name="ProjectFile">Target project file, relative to the target directory.</param>
/// <param name="PackageId">Package to align.</param>
/// <param name="CurrentVersion">Observed version, or <c>(unpinned)</c> when none is declared.</param>
/// <param name="ProposedVersion">Suggested exact version.</param>
/// <param name="Reason">Why the change is proposed.</param>
public sealed record ProposedEdit(
    string ProjectFile,
    string PackageId,
    string CurrentVersion,
    string ProposedVersion,
    string Reason);
