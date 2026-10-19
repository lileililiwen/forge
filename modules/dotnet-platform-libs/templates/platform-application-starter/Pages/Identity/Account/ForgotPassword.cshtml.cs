using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Identity;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Mvc.RazorPages;
using StarterApp.SiteUsers;

namespace StarterApp.Pages.Identity.Account;

[AllowAnonymous]
public sealed class ForgotPasswordModel : PageModel
{
    private readonly UserManager<ApplicationUser> _userManager;

    public ForgotPasswordModel(UserManager<ApplicationUser> userManager)
    {
        _userManager = userManager;
    }

    [BindProperty]
    public InputModel Input { get; set; } = new();

    public string? Message { get; set; }

    public sealed class InputModel
    {
        public string Email { get; set; } = string.Empty;
    }

    public void OnGet()
    {
    }

    public async Task<IActionResult> OnPostAsync()
    {
        if (string.IsNullOrWhiteSpace(Input.Email))
        {
            Message = "An email is required.";
            return Page();
        }

        // Identical response whether or not the email matches a known user.
        var user = await _userManager.FindByEmailAsync(Input.Email);
        if (user is not null)
        {
            var token = await _userManager.GeneratePasswordResetTokenAsync(user);
            // The application owner wires a delivery channel here (mail, in-app, etc.).
            // The platform never owns an application delivery channel.
            _ = token;
        }

        Message = "If a matching account exists, a recovery flow has been started.";
        return Page();
    }
}
