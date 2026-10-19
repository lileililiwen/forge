using Platform.Core.Results;

namespace Platform.Domain.Tests;

public class DomainErrorTests
{
    [Fact]
    public void DomainException_carries_stable_error()
    {
        var error = new Error("orders.closed", "The order is closed.");

        var exception = new DomainException(error);

        Assert.Same(error, exception.Error);
        Assert.Equal("The order is closed.", exception.Message);
    }

    [Fact]
    public void DomainException_preserves_inner_exception_without_surfacing_it()
    {
        var error = new Error("orders.closed", "The order is closed.");
        var inner = new InvalidOperationException("storage timeout");

        var exception = new DomainException(error, inner);

        Assert.Same(inner, exception.InnerException);
        Assert.Equal("The order is closed.", exception.Message);
        Assert.DoesNotContain("storage timeout", exception.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void DomainException_rejects_null_error()
    {
        Assert.Throws<ArgumentNullException>(() => new DomainException(null!));
        Assert.Throws<ArgumentNullException>(() => new DomainException(null!, new InvalidOperationException()));
        Assert.Throws<ArgumentNullException>(() => new DomainException(new Error("x", "y"), null!));
    }

    [Fact]
    public void Validation_exception_carries_stable_validation_code()
    {
        var exception = new DomainValidationException("Quantity must be positive.");

        Assert.Equal(Error.ValidationCode, exception.Error.Code);
        Assert.Equal("Quantity must be positive.", exception.Error.Message);
        Assert.Equal("Quantity must be positive.", exception.Message);
    }

    [Fact]
    public void NotFound_exception_carries_stable_not_found_code()
    {
        var exception = new DomainNotFoundException("Order was not found.");

        Assert.Equal(Error.NotFoundCode, exception.Error.Code);
        Assert.Equal("Order was not found.", exception.Message);
    }

    [Fact]
    public void Conflict_exception_carries_stable_conflict_code()
    {
        var exception = new DomainConflictException("The order was already submitted.");

        Assert.Equal(DomainErrorCodes.Conflict, exception.Error.Code);
        Assert.Equal("The order was already submitted.", exception.Message);
    }

    [Fact]
    public void Web_adapter_maps_domain_error_without_inspecting_internals()
    {
        DomainException exception = new DomainNotFoundException("Order was not found.");

        var statusCode = MapToStatusCode(exception.Error);

        Assert.Equal(404, statusCode);
        Assert.Equal(Error.NotFoundCode, exception.Error.Code);
    }

    private static int MapToStatusCode(Error error) => error.Code switch
    {
        Error.ValidationCode => 400,
        Error.NotFoundCode => 404,
        _ => 500,
    };
}
