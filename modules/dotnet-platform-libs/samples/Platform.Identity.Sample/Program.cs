using Platform.Identity.Contracts;
using Platform.Identity.Sample;

var builder = WebApplication.CreateBuilder(args);

// Stage 3: identity. The platform owns the credential/result contracts;
// the application owns the user store, the secret handling, and the
// session or token issuance that follows a successful verification.
builder.Services.AddSingleton<ICredentialVerifier>(new SampleCredentialVerifier()
    .Add("ada", "correct-horse", new CurrentUser("user-ada", "ada@example.com", Permissions: new List<string> { "sample.read" })));

var app = builder.Build();
app.MapPost("/sample/login", async (LoginRequest request, ICredentialVerifier verifier, CancellationToken cancellationToken) =>
{
    if (!new Credential(request.Identifier, request.Secret).IsValid)
    {
        return Results.BadRequest(new { authenticated = false });
    }

    var result = await verifier.VerifyAsync(new Credential(request.Identifier, request.Secret), cancellationToken);
    return result.Succeeded
        ? Results.Json(new { authenticated = true, subject = result.Value!.SubjectId })
        : Results.Json(new { authenticated = false }, statusCode: StatusCodes.Status401Unauthorized);
});
app.Run();

/// <summary>Entry point marker for matrix hosting.</summary>
public partial class Program;
