using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Identity;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Mvc.RazorPages;
using Microsoft.Extensions.Options;
using StarterApp.SiteUsers;

namespace StarterApp.Pages.Identity.Account;

[AllowAnonymous]
public sealed class RegisterModel : PageModel
{
    /// <summary>
    /// One message for every refused submission. A password mismatch, a password the
    /// policy rejects and an email the store already knows all return this exact text,
    /// so the response cannot be used to tell those cases apart or to learn whether an
    /// address is registered.
    /// </summary>
    private const string GenericRefusal = "Registration could not be completed. Check the details and try again.";

    /// <summary>Shown while the application owner has not enabled self-registration.</summary>
    private const string ClosedMessage = "This site does not accept public registrations. Ask the site owner for an account.";

    private readonly UserManager<ApplicationUser> _userManager;
    private readonly IOptions<SiteUsersOptions> _siteOptions;

    public RegisterModel(
        UserManager<ApplicationUser> userManager,
        IOptions<SiteUsersOptions> siteOptions)
    {
        _userManager = userManager;
        _siteOptions = siteOptions;
    }

    [BindProperty]
    public InputModel Input { get; set; } = new();

    public string? Message { get; set; }

    /// <summary>
    /// False unless the owner set <c>SiteUsers:SelfRegistrationEnabled</c>. The view
    /// renders no form at all when it is false, and the POST handler re-checks it.
    /// </summary>
    public bool IsRegistrationEnabled => _siteOptions.Value.SelfRegistrationEnabled;

    public sealed class InputModel
    {
        public string Email { get; set; } = string.Empty;
        public string Password { get; set; } = string.Empty;
        public string ConfirmPassword { get; set; } = string.Empty;
    }

    public void OnGet()
    {
        if (!IsRegistrationEnabled)
        {
            Message = ClosedMessage;
        }
    }

    public async Task<IActionResult> OnPostAsync()
    {
        // Self-registration is off until the application owner turns it on. The check
        // runs here as well as in the view so posting the route directly while the
        // page refuses to render a form still refuses.
        if (!IsRegistrationEnabled)
        {
            Message = ClosedMessage;
            return Page();
        }

        if (string.IsNullOrWhiteSpace(Input.Email)
            || string.IsNullOrEmpty(Input.Password)
            || !string.Equals(Input.Password, Input.ConfirmPassword, StringComparison.Ordinal))
        {
            Message = GenericRefusal;
            return Page();
        }

        var user = new ApplicationUser
        {
            UserName = Input.Email,
            Email = Input.Email,
        };

        // The account is created through the same application-owned user store every
        // other identity operation uses. The platform owns no user entity and no
        // migration, and the application owner wires confirmation or sign-in here.
        var result = await _userManager.CreateAsync(user, Input.Password);
        if (!result.Succeeded)
        {
            // Password policy, duplicate email and duplicate user name all land here
            // and all return the same text.
            Message = GenericRefusal;
            return Page();
        }

        Message = "The account was created. You can sign in now.";
        return Page();
    }
}