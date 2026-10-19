using Platform.Privacy.Contracts;

namespace Platform.Privacy.Evaluation;

/// <summary>Versioned consent policy. Evaluation fails closed.</summary>
/// <param name="CurrentVersion">Policy version; records under older versions lose optional grants.</param>
public sealed class ConsentPolicy(int CurrentVersion)
{
    /// <summary>Evaluates a category against an optional stored record.</summary>
    public bool IsGranted(ConsentCategory category, ConsentRecord? record)
    {
        if (category == ConsentCategory.Necessary)
        {
            return true;
        }

        if (record is null || record.PolicyVersion != CurrentVersion)
        {
            return false;
        }

        return record.Granted.Contains(category);
    }
}
