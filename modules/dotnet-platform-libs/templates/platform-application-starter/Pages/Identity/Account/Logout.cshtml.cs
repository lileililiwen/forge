using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Identity;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Mvc.RazorPages;
using StarterApp.SiteUsers;

namespace StarterApp.Pages.Identity.Account;

[AllowAnonymous]
public sealed class LogoutModel : PageModel
{
    private readonly SignInManager<ApplicationUser> _signInManager;

    public LogoutModel(SignInManager<ApplicationUser> signInManager)
    {
        _signInManager = signInManager;
    }

    public IActionResult OnGet()
    {
        // Browsers hit /Identity/Account/Logout as GET from the partial form action.
        // The partial uses POST with antiforgery, so this GET returns a confirmation page.
        return Page();
    }

    public async Task<IActionResult> OnPostAsync()
    {
        await _signInManager.SignOutAsync();
        return Redirect("/Identity/Account/Login");
    }
}
