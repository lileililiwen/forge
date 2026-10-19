namespace StarterApp.SiteUsers;

/// <summary>Application-owned configuration for the site-user feature.</summary>
public sealed class SiteUsersOptions
{
    /// <summary>Configuration section name.</summary>
    public const string SectionName = "SiteUsers";

    /// <summary>Cookie sub-section name.</summary>
    public const string CookieSectionName = "Cookie";

    /// <summary>Master switch for the site-user feature at runtime.</summary>
    public bool Enabled { get; set; } = true;

    /// <summary>Opt-in self-registration. Defaults to false to avoid anonymous sign-ups.</summary>
    public bool SelfRegistrationEnabled { get; set; }

    /// <summary>Application-owned authentication scheme name.</summary>
    public string AuthenticationScheme { get; set; } = "SiteUsers";

    /// <summary>Cookie configuration owned by the application.</summary>
    public SiteUsersCookieOptions Cookie { get; set; } = new();

    /// <summary>True when the feature is configured for a Development-only environment.</summary>
    public bool DevelopmentOnly { get; set; }
}

/// <summary>Cookie options owned by the application.</summary>
public sealed class SiteUsersCookieOptions
{
    /// <summary>Cookie name. Defaults to the application scheme.</summary>
    public string? Name { get; set; }

    /// <summary>Cookie secure policy. Allowed values: <c>Always</c>, <c>SameAsRequest</c>, <c>None</c>.</summary>
    public string SecurePolicy { get; set; } = "SameAsRequest";

    /// <summary>Cookie same-site mode. Allowed values: <c>Strict</c>, <c>Lax</c>, <c>None</c>.</summary>
    public string SameSite { get; set; } = "Lax";

    /// <summary>Cookie lifetime in minutes.</summary>
    public int LifetimeMinutes { get; set; } = 60;

    /// <summary>Login path used by the cookie middleware.</summary>
    public string LoginPath { get; set; } = "/Identity/Account/Login";

    /// <summary>Access-denied path used by the cookie middleware.</summary>
    public string AccessDeniedPath { get; set; } = "/Identity/Account/AccessDenied";

    /// <summary>Logout path used by the cookie middleware.</summary>
    public string LogoutPath { get; set; } = "/Identity/Account/Logout";

    /// <summary>Return-url parameter expected by the cookie middleware.</summary>
    public string ReturnUrlParameter { get; set; } = "ReturnUrl";
}
