using System.Text.Json;

namespace Platform.ConsumerConformance;

/// <summary>
/// Validates the canonical shared-contract envelopes. Compatible documents are
/// accepted; documents with a missing required field or an incompatible status
/// vocabulary fail with the contract and field identified.
/// </summary>
public sealed class ContractEnvelopeConformanceTests
{
    public static TheoryData<string> Contracts() => new()
    {
        "identity-subject",
        "permission",
        "tenant",
        "audit",
        "gate-result",
        "release-evidence",
    };

    [Theory]
    [MemberData(nameof(Contracts))]
    public void Compatible_envelope_is_accepted(string contract)
    {
        using var document = ContractFixtureReader.Load(ContractsDirectory(), contract, "valid");
        var result = ContractFixtureReader.Validate(contract, document);
        Assert.True(result.Accepted, $"Fixture {contract}.valid.json must be accepted but got: {result.Message}");
        Assert.Equal(contract, result.Contract);
    }

    [Theory]
    [MemberData(nameof(Contracts))]
    public void Incompatible_envelope_fails_with_contract_and_field_identified(string contract)
    {
        using var document = ContractFixtureReader.Load(ContractsDirectory(), contract, "invalid");
        var result = ContractFixtureReader.Validate(contract, document);
        Assert.False(result.Accepted, $"Fixture {contract}.invalid.json must be rejected.");
        Assert.Equal(contract, result.Contract);
        Assert.False(string.IsNullOrWhiteSpace(result.Field), "The offending field must be identified.");
        Assert.False(string.IsNullOrWhiteSpace(result.Message), "A failure message must be provided.");
    }

    [Fact]
    public void Missing_subject_is_rejected_on_the_subject_field()
    {
        using var document = ContractFixtureReader.Load(ContractsDirectory(), "identity-subject", "invalid");
        var result = ContractFixtureReader.Validate("identity-subject", document);
        Assert.False(result.Accepted);
        Assert.Equal("subjectId", result.Field);
    }

    [Fact]
    public void Unknown_status_vocabulary_is_rejected_on_the_status_field()
    {
        using var document = ContractFixtureReader.Load(ContractsDirectory(), "permission", "invalid");
        var result = ContractFixtureReader.Validate("permission", document);
        Assert.False(result.Accepted);
        Assert.Equal("effect", result.Field);
    }

    [Fact]
    public void Unknown_contract_is_rejected()
    {
        using var document = JsonDocument.Parse("""{"contract": "platform.unknown/v1"}""");
        var result = ContractFixtureReader.Validate("unknown", document);
        Assert.False(result.Accepted);
        Assert.Equal("unknown", result.Contract);
    }

    private static string ContractsDirectory()
    {
        var current = new DirectoryInfo(AppContext.BaseDirectory);
        while (current is not null)
        {
            var candidate = Path.Combine(current.FullName, "Fixtures", "contracts");
            if (Directory.Exists(candidate)
                && File.Exists(Path.Combine(candidate, "identity-subject.valid.json")))
            {
                return candidate;
            }

            current = current.Parent;
        }

        throw new InvalidOperationException("Unable to locate the Fixtures/contracts directory.");
    }
}
