using System.Text.Json;

namespace Platform.ConsumerConformance;

/// <summary>
/// Deterministic reader for the canonical shared-contract fixtures in
/// <c>Fixtures/contracts</c>. Validates required fields and status vocabularies
/// without any runtime repository coupling: fixtures are local JSON files and
/// the contract repository is never a package dependency.
/// </summary>
public static class ContractFixtureReader
{
    /// <summary>Loads a fixture document from the contracts directory.</summary>
    public static JsonDocument Load(string contractsDirectory, string contract, string variant)
    {
        var path = Path.Combine(contractsDirectory, $"{contract}.{variant}.json");
        return JsonDocument.Parse(File.ReadAllText(path));
    }

    /// <summary>
    /// Validates a contract document. Accepted documents satisfy every required
    /// field and status vocabulary; rejected documents identify the contract
    /// and the offending field.
    /// </summary>
    public static ContractConformanceResult Validate(string contract, JsonDocument document)
    {
        var root = document.RootElement;
        return contract switch
        {
            "identity-subject" => ValidateIdentitySubject(root),
            "permission" => ValidatePermission(root),
            "tenant" => ValidateTenant(root),
            "audit" => ValidateAudit(root),
            "gate-result" => ValidateGateResult(root),
            "release-evidence" => ValidateReleaseEvidence(root),
            _ => ContractConformanceResult.Reject(contract, "(contract)", $"Unknown contract '{contract}'."),
        };
    }

    private static ContractConformanceResult ValidateIdentitySubject(JsonElement root)
    {
        const string contract = "identity-subject";
        if (!IsContract(root, "platform.identity.subject/v1"))
            return ContractConformanceResult.Reject(contract, "contract", "Expected contract 'platform.identity.subject/v1'.");
        if (!HasNonEmptyString(root, "subjectId"))
            return ContractConformanceResult.Reject(contract, "subjectId", "A non-empty subjectId is required.");
        return ContractConformanceResult.Accept(contract);
    }

    private static ContractConformanceResult ValidatePermission(JsonElement root)
    {
        const string contract = "permission";
        if (!IsContract(root, "platform.authorization.permission/v1"))
            return ContractConformanceResult.Reject(contract, "contract", "Expected contract 'platform.authorization.permission/v1'.");
        if (!HasNonEmptyString(root, "resource"))
            return ContractConformanceResult.Reject(contract, "resource", "A non-empty resource is required.");
        if (!HasNonEmptyString(root, "action"))
            return ContractConformanceResult.Reject(contract, "action", "A non-empty action is required.");
        if (!IsOneOf(root, "effect", ["allow", "deny"]))
            return ContractConformanceResult.Reject(contract, "effect", "Effect must be 'allow' or 'deny'.");
        return ContractConformanceResult.Accept(contract);
    }

    private static ContractConformanceResult ValidateTenant(JsonElement root)
    {
        const string contract = "tenant";
        if (!IsContract(root, "platform.tenancy.operation/v1"))
            return ContractConformanceResult.Reject(contract, "contract", "Expected contract 'platform.tenancy.operation/v1'.");
        if (!HasNonEmptyString(root, "operationId"))
            return ContractConformanceResult.Reject(contract, "operationId", "A non-empty operationId is required.");
        if (!IsOneOf(root, "state", ["pending", "running", "succeeded", "failed"]))
            return ContractConformanceResult.Reject(contract, "state", "State must be 'pending', 'running', 'succeeded', or 'failed'.");
        return ContractConformanceResult.Accept(contract);
    }

    private static ContractConformanceResult ValidateAudit(JsonElement root)
    {
        const string contract = "audit";
        if (!IsContract(root, "platform.audit.event/v1"))
            return ContractConformanceResult.Reject(contract, "contract", "Expected contract 'platform.audit.event/v1'.");
        if (!HasNonEmptyString(root, "action"))
            return ContractConformanceResult.Reject(contract, "action", "A non-empty action is required.");
        if (!IsOneOf(root, "severity", ["debug", "info", "warning", "error", "critical"]))
            return ContractConformanceResult.Reject(contract, "severity", "Severity must be 'debug', 'info', 'warning', 'error', or 'critical'.");
        return ContractConformanceResult.Accept(contract);
    }

    private static ContractConformanceResult ValidateGateResult(JsonElement root)
    {
        const string contract = "gate-result";
        if (!IsContract(root, "platform.release.gate/v1"))
            return ContractConformanceResult.Reject(contract, "contract", "Expected contract 'platform.release.gate/v1'.");
        if (!HasNonEmptyString(root, "gate"))
            return ContractConformanceResult.Reject(contract, "gate", "A non-empty gate name is required.");
        if (!IsOneOf(root, "status", ["pass", "fail", "blocked"]))
            return ContractConformanceResult.Reject(contract, "status", "Status must be 'pass', 'fail', or 'blocked'.");
        return ContractConformanceResult.Accept(contract);
    }

    private static ContractConformanceResult ValidateReleaseEvidence(JsonElement root)
    {
        const string contract = "release-evidence";
        if (!IsContract(root, "platform.release.evidence/v1"))
            return ContractConformanceResult.Reject(contract, "contract", "Expected contract 'platform.release.evidence/v1'.");
        if (!HasNonEmptyString(root, "packageId"))
            return ContractConformanceResult.Reject(contract, "packageId", "A non-empty packageId is required.");
        if (!root.TryGetProperty("evidence", out var evidence) || evidence.ValueKind != JsonValueKind.Array || evidence.GetArrayLength() == 0)
            return ContractConformanceResult.Reject(contract, "evidence", "At least one evidence entry is required; unavailable evidence is not a pass.");
        var index = 0;
        foreach (var entry in evidence.EnumerateArray())
        {
            if (!HasNonEmptyString(entry, "kind"))
                return ContractConformanceResult.Reject(contract, $"evidence[{index}].kind", "Each evidence entry requires a kind.");
            if (!IsOneOf(entry, "status", ["available", "unavailable"]))
                return ContractConformanceResult.Reject(contract, $"evidence[{index}].status", "Evidence status must be 'available' or 'unavailable'.");
            index++;
        }
        return ContractConformanceResult.Accept(contract);
    }

    private static bool IsContract(JsonElement root, string expected) =>
        root.TryGetProperty("contract", out var value)
        && value.ValueKind == JsonValueKind.String
        && string.Equals(value.GetString(), expected, StringComparison.Ordinal);

    private static bool HasNonEmptyString(JsonElement root, string name) =>
        root.TryGetProperty(name, out var value)
        && value.ValueKind == JsonValueKind.String
        && !string.IsNullOrWhiteSpace(value.GetString());

    private static bool IsOneOf(JsonElement root, string name, string[] vocabulary) =>
        root.TryGetProperty(name, out var value)
        && value.ValueKind == JsonValueKind.String
        && vocabulary.Contains(value.GetString(), StringComparer.Ordinal);
}

/// <summary>Outcome of a single contract-fixture validation.</summary>
public sealed record ContractConformanceResult(
    bool Accepted,
    string Contract,
    string? Field = null,
    string? Message = null)
{
    /// <summary>Creates an accepted outcome.</summary>
    public static ContractConformanceResult Accept(string contract) => new(true, contract);

    /// <summary>Creates a rejected outcome identifying the contract and field.</summary>
    public static ContractConformanceResult Reject(string contract, string field, string message) =>
        new(false, contract, field, message);
}
