using System.Net;
using System.Net.Sockets;
using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Security;

/// <summary>Default SSRF-safe target validator. Requires HTTPS, rejects loopback/private/link-local destinations, and supports an optional allow-list.</summary>
public sealed class SsrfTargetValidator : ISsrfTargetValidator
{
    private readonly WebhookOptions _options;

    /// <summary>Creates the validator.</summary>
    public SsrfTargetValidator(WebhookOptions options)
    {
        _options = options ?? throw new ArgumentNullException(nameof(options));
        _options.Validate();
    }

    /// <inheritdoc />
    public WebhookTargetValidation Validate(Uri target)
    {
        ArgumentNullException.ThrowIfNull(target);
        if (!target.IsAbsoluteUri) return WebhookTargetValidation.Reject(WebhookTargetRejection.NotAbsolute);
        if (!string.Equals(target.Scheme, Uri.UriSchemeHttps, StringComparison.OrdinalIgnoreCase)) return WebhookTargetValidation.Reject(WebhookTargetRejection.InsecureScheme);
        if (_options.TargetAllowList.Count > 0 && !IsAllowListed(target)) return WebhookTargetValidation.Reject(WebhookTargetRejection.NotAllowListed);
        if (IPAddress.TryParse(target.Host, out var literal))
        {
            return ClassifyAddress(literal);
        }
        try
        {
            var addresses = Dns.GetHostAddresses(target.Host);
            if (addresses is null || addresses.Length == 0) return WebhookTargetValidation.Reject(WebhookTargetRejection.NoAddresses);
            foreach (var address in addresses)
            {
                var result = ClassifyAddress(address);
                if (!result.Allowed) return result;
            }
        }
        catch (SocketException)
        {
            return WebhookTargetValidation.Reject(WebhookTargetRejection.NoAddresses);
        }
        return WebhookTargetValidation.Success();
    }

    private WebhookTargetValidation ClassifyAddress(IPAddress address)
    {
        if (IPAddress.IsLoopback(address))
        {
            return _options.AllowLoopbackTargets ? WebhookTargetValidation.Success() : WebhookTargetValidation.Reject(WebhookTargetRejection.Loopback);
        }
        if (IsLinkLocal(address)) return WebhookTargetValidation.Reject(WebhookTargetRejection.LinkLocal);
        if (IsPrivate(address)) return WebhookTargetValidation.Reject(WebhookTargetRejection.PrivateNetwork);
        return WebhookTargetValidation.Success();
    }

    private static bool IsLinkLocal(IPAddress address)
    {
        if (address.AddressFamily == AddressFamily.InterNetwork) return (address.GetAddressBytes()[0] & 0xF0) == 0xA0;
        return address.GetAddressBytes()[0] == 0xFE && (address.GetAddressBytes()[1] & 0xC0) == 0x80;
    }

    private static bool IsPrivate(IPAddress address)
    {
        if (address.AddressFamily == AddressFamily.InterNetwork)
        {
            var bytes = address.GetAddressBytes();
            if (bytes[0] == 10) return true;
            if (bytes[0] == 172 && bytes[1] >= 16 && bytes[1] <= 31) return true;
            if (bytes[0] == 192 && bytes[1] == 168) return true;
            if (bytes[0] == 169 && bytes[1] == 254) return true;
            if (bytes[0] == 100 && bytes[1] >= 64 && bytes[1] <= 127) return true;
            return false;
        }
        var ipv6 = address.GetAddressBytes();
        if ((ipv6[0] & 0xFE) == 0xFC) return true;
        if (ipv6[0] == 0xFC || ipv6[0] == 0xFD) return true;
        return false;
    }

    private bool IsAllowListed(Uri target)
    {
        foreach (var entry in _options.TargetAllowList)
        {
            if (string.IsNullOrWhiteSpace(entry)) continue;
            if (entry.Contains('/'))
            {
                if (TryMatchCidr(entry, target)) return true;
            }
            else if (string.Equals(entry, target.Host, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
        }
        return false;
    }

    private static bool TryMatchCidr(string cidr, Uri target)
    {
        var parts = cidr.Split('/');
        if (parts.Length != 2) return false;
        if (!IPAddress.TryParse(parts[0], out var network)) return false;
        if (!int.TryParse(parts[1], out var prefix)) return false;
        if (IPAddress.TryParse(target.Host, out var literal))
        {
            return IsInCidr(literal, network, prefix);
        }
        try
        {
            var addresses = Dns.GetHostAddresses(target.Host);
            return addresses.Any(address => IsInCidr(address, network, prefix));
        }
        catch (SocketException)
        {
            return false;
        }
    }

    private static bool IsInCidr(IPAddress address, IPAddress network, int prefix)
    {
        var addressBytes = address.GetAddressBytes();
        var networkBytes = network.GetAddressBytes();
        if (addressBytes.Length != networkBytes.Length) return false;
        var maxPrefix = addressBytes.Length * 8;
        if (prefix < 0 || prefix > maxPrefix) return false;
        var fullBytes = prefix / 8;
        for (var i = 0; i < fullBytes; i++)
        {
            if (addressBytes[i] != networkBytes[i]) return false;
        }
        if (fullBytes == addressBytes.Length) return true;
        var remainingBits = prefix % 8;
        var mask = (byte)(0xFF << (8 - remainingBits));
        return (addressBytes[fullBytes] & mask) == (networkBytes[fullBytes] & mask);
    }
}
