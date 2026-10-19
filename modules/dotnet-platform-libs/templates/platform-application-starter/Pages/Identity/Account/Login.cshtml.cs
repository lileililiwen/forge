using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Identity;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Mvc.RazorPages;
using Microsoft.Extensions.Options;
using StarterApp.SiteUsers;

namespace StarterApp.Pages.Identity.Account;

[AllowAnonymous]
public sealed class LoginModel : PageModel
{
    private readonly SignInManager<ApplicationUser> _signInManager;
    private readonly IOptions<SiteUsersOptions> _siteOptions;

    public LoginModel(
        SignInManager<ApplicationUser> signInManager,
        IOptions<SiteUsersOptions> siteOptions)
    {
        _signInManager = signInManager;
        _siteOptions = siteOptions;
    }

    [BindProperty]
    public InputModel Input { get; set; } = new();

    public string? ErrorMessage { get; set; }

    public string? ReturnUrl { get; set; }

    public sealed class InputModel
    {
        public string Email { get; set; } = string.Empty;
        public string Password { get; set; } = string.Empty;
    }

    public void OnGet(string? returnUrl = null)
    {
        ReturnUrl = returnUrl;
    }

    public async Task<IActionResult> OnPostAsync(string? returnUrl = null)
    {
        ReturnUrl = returnUrl;

        if (!string.IsNullOrEmpty(Input.Email) && !string.IsNullOrEmpty(Input.Password))
        {
            var result = await _signInManager.PasswordSignInAsync(Input.Email, Input.Password, isPersistent: false, lockoutOnFailure: true);
            if (result.Succeeded)
            {
                if (!string.IsNullOrEmpty(ReturnUrl) && Url.IsLocalUrl(ReturnUrl))
                {
                    return LocalRedirect(ReturnUrl);
                }
                return Redirect("/");
            }
            if (result.IsLockedOut)
            {
                // Identical generic message for unknown email, wrong password, and lockout.
                ErrorMessage = "Invalid sign-in. Please try again.";
            }
            else
            {
                ErrorMessage = "Invalid sign-in. Please try again.";
            }
        }
        else
        {
            ErrorMessage = "Email and password are required.";
        }

        return Page();
    }
}
