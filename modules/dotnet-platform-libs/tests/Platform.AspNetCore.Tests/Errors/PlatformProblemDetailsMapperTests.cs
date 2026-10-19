using Microsoft.AspNetCore.Http;
using Platform.AspNetCore.Errors;
using Platform.Core.Results;

namespace Platform.AspNetCore.Tests.Errors;

public class PlatformProblemDetailsMapperTests
{
    [Fact]
    public void Validation_error_maps_to_400_with_validation_category()
    {
        var mapper = new PlatformProblemDetailsMapper();
        var error = Error.Validation("name is required");

        var problem = mapper.Map(error);

        Assert.Equal(StatusCodes.Status400BadRequest, problem.Status);
        Assert.Equal("validation", problem.Title);
        Assert.Equal("name is required", problem.Detail);
        Assert.Equal("https://platform.example/problems/validation", problem.Type);
        Assert.Equal("platform.validation", problem.Extensions["code"]);
    }

    [Fact]
    public void NotFound_error_maps_to_404_with_not_found_category()
    {
        var mapper = new PlatformProblemDetailsMapper();
        var error = Error.NotFound("user not found");

        var problem = mapper.Map(error);

        Assert.Equal(StatusCodes.Status404NotFound, problem.Status);
        Assert.Equal("not_found", problem.Title);
        Assert.Equal("user not found", problem.Detail);
        Assert.Equal("https://platform.example/problems/not_found", problem.Type);
        Assert.Equal("platform.not_found", problem.Extensions["code"]);
    }

    [Fact]
    public void Unknown_code_maps_to_500_with_default_category()
    {
        var mapper = new PlatformProblemDetailsMapper();
        var error = new Error("platform.something.weird", "boom");

        var problem = mapper.Map(error);

        Assert.Equal(StatusCodes.Status500InternalServerError, problem.Status);
        Assert.Equal("something.weird", problem.Title);
        Assert.Equal("boom", problem.Detail);
    }

    [Fact]
    public void Metadata_is_copied_into_extensions()
    {
        var mapper = new PlatformProblemDetailsMapper();
        var metadata = new Dictionary<string, object?>
        {
            ["field"] = "email",
            ["attempt"] = 3,
        };
        var error = new Error("platform.validation", "bad email", metadata);

        var problem = mapper.Map(error);

        Assert.Equal("email", problem.Extensions["field"]);
        Assert.Equal(3, problem.Extensions["attempt"]);
    }

    [Fact]
    public void StatusCodeFor_validation_returns_400()
    {
        var mapper = new PlatformProblemDetailsMapper();

        Assert.Equal(StatusCodes.Status400BadRequest, mapper.StatusCodeFor(Error.Validation("x")));
    }

    [Fact]
    public void StatusCodeFor_not_found_returns_404()
    {
        var mapper = new PlatformProblemDetailsMapper();

        Assert.Equal(StatusCodes.Status404NotFound, mapper.StatusCodeFor(Error.NotFound("x")));
    }

    [Fact]
    public void StatusCodeFor_unknown_returns_500()
    {
        var mapper = new PlatformProblemDetailsMapper();

        Assert.Equal(StatusCodes.Status500InternalServerError, mapper.StatusCodeFor(new Error("platform.other", "x")));
    }

    [Fact]
    public void Map_throws_for_null_error()
    {
        var mapper = new PlatformProblemDetailsMapper();

        Assert.Throws<ArgumentNullException>(() => mapper.Map(null!));
    }

    [Fact]
    public void StatusCodeFor_throws_for_null_error()
    {
        var mapper = new PlatformProblemDetailsMapper();

        Assert.Throws<ArgumentNullException>(() => mapper.StatusCodeFor(null!));
    }
}
