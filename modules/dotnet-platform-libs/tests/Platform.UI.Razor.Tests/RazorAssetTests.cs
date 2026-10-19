using Platform.UI.Razor;

namespace Platform.UI.Razor.Tests;

public sealed class RazorAssetTests
{
    [Fact]
    public void Static_assets_expose_shared_theme_and_focus_contract()
    {
        var css = File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "platform-ui.css"));
        var tokens = File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "platform-tokens.css"));
        Assert.NotNull(typeof(PlatformUiRazorAssemblyMarker));
        Assert.Contains("--color-primary", tokens);
        Assert.Contains("data-theme=\"dark\"", tokens);
        Assert.Contains(":focus-visible", css);
        Assert.Contains("aria-invalid", css);
    }
}
