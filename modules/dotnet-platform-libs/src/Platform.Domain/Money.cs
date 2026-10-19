namespace Platform.Domain;

/// <summary>
/// Validated money value object with a decimal amount and a normalized
/// currency code. Currency text is trimmed and normalized to uppercase
/// invariant form. Arithmetic across different currencies is rejected;
/// exchange-rate conversion and rounding policy remain application-owned.
/// </summary>
public sealed record Money
{
    /// <summary>
    /// Initializes a new instance of the <see cref="Money"/> class.
    /// </summary>
    /// <param name="amount">The monetary amount.</param>
    /// <param name="currency">The currency code. Leading and trailing whitespace is removed and the remainder is normalized to uppercase invariant form.</param>
    public Money(decimal amount, string currency)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(currency);
        Amount = amount;
        Currency = currency.Trim().ToUpperInvariant();
    }

    /// <summary>
    /// Gets the monetary amount.
    /// </summary>
    public decimal Amount { get; init; }

    /// <summary>
    /// Gets the normalized currency code.
    /// </summary>
    public string Currency { get; init; }

    /// <summary>
    /// Creates a zero amount in the specified currency.
    /// </summary>
    /// <param name="currency">The currency code.</param>
    /// <returns>A zero <see cref="Money"/> value.</returns>
    public static Money Zero(string currency = "USD") => new(0m, currency);

    /// <summary>
    /// Adds two money values of the same currency.
    /// </summary>
    /// <param name="other">The value to add.</param>
    /// <returns>The sum.</returns>
    public Money Add(Money other)
    {
        ArgumentNullException.ThrowIfNull(other);
        EnsureSameCurrency(other);
        return this with { Amount = Amount + other.Amount };
    }

    /// <summary>
    /// Subtracts a money value of the same currency.
    /// </summary>
    /// <param name="other">The value to subtract.</param>
    /// <returns>The difference.</returns>
    public Money Subtract(Money other)
    {
        ArgumentNullException.ThrowIfNull(other);
        EnsureSameCurrency(other);
        return this with { Amount = Amount - other.Amount };
    }

    /// <summary>
    /// Multiplies the amount by a scalar factor. The currency is unchanged.
    /// </summary>
    /// <param name="factor">The multiplication factor.</param>
    /// <returns>The scaled value.</returns>
    public Money Multiply(decimal factor) => this with { Amount = Amount * factor };

    /// <summary>
    /// Adds two money values of the same currency.
    /// </summary>
    public static Money operator +(Money left, Money right)
    {
        ArgumentNullException.ThrowIfNull(left);
        return left.Add(right);
    }

    /// <summary>
    /// Subtracts two money values of the same currency.
    /// </summary>
    public static Money operator -(Money left, Money right)
    {
        ArgumentNullException.ThrowIfNull(left);
        return left.Subtract(right);
    }

    /// <summary>
    /// Multiplies a money value by a scalar factor.
    /// </summary>
    public static Money operator *(Money left, decimal right)
    {
        ArgumentNullException.ThrowIfNull(left);
        return left.Multiply(right);
    }

    private void EnsureSameCurrency(Money other)
    {
        if (!string.Equals(Currency, other.Currency, StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                $"Cannot operate on Money with different currencies: {Currency} and {other.Currency}.");
        }
    }
}
