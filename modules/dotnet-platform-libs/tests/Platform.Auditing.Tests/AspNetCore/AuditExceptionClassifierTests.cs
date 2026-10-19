using System.Data.Common;
using System.Net.Http;
using System.Net.Sockets;
using Platform.Auditing.AspNetCore.Capture;

namespace Platform.Auditing.Tests.AspNetCore;

public class AuditExceptionClassifierTests
{
    [Theory]
    [InlineData(typeof(ArgumentException), AuditExceptionClassification.Validation)]
    [InlineData(typeof(FormatException), AuditExceptionClassification.Validation)]
    [InlineData(typeof(InvalidDataException), AuditExceptionClassification.Validation)]
    [InlineData(typeof(KeyNotFoundException), AuditExceptionClassification.NotFound)]
    [InlineData(typeof(FileNotFoundException), AuditExceptionClassification.NotFound)]
    [InlineData(typeof(SocketException), AuditExceptionClassification.Dependency)]
    [InlineData(typeof(HttpRequestException), AuditExceptionClassification.Dependency)]
    [InlineData(typeof(TestDbException), AuditExceptionClassification.Dependency)]
    [InlineData(typeof(TimeoutException), AuditExceptionClassification.Timeout)]
    [InlineData(typeof(TaskCanceledException), AuditExceptionClassification.Timeout)]
    [InlineData(typeof(OperationCanceledException), AuditExceptionClassification.Timeout)]
    [InlineData(typeof(NullReferenceException), AuditExceptionClassification.ServerError)]
    public void Classifies_known_exceptions_into_safe_kinds(Type exceptionType, string expectedKind)
    {
        var exception = (Exception)Activator.CreateInstance(exceptionType)!;
        var classification = AuditExceptionClassifier.Classify(exception);
        Assert.Equal(expectedKind, classification.Kind);
    }

    [Fact]
    public void Classification_does_not_expose_exception_message()
    {
        var exception = new InvalidOperationException("secret-internal-detail-token-xyz");
        var classification = AuditExceptionClassifier.Classify(exception);

        Assert.DoesNotContain("secret-internal-detail", classification.Kind, StringComparison.Ordinal);
        Assert.Equal(AuditExceptionClassification.ServerError, classification.Kind);
    }

    [Fact]
    public void Unknown_exception_falls_back_to_server_error()
    {
        var classification = AuditExceptionClassifier.Classify(new Exception("anything"));
        Assert.Equal(AuditExceptionClassification.ServerError, classification.Kind);
    }
}

/// <summary>Concrete <see cref="DbException"/> for classifier tests (the base type is abstract).</summary>
internal sealed class TestDbException : DbException
{
}
