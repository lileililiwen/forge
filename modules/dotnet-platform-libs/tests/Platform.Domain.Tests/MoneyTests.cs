namespace Platform.Domain.Tests;

public class MoneyTests
{
    [Fact]
    public void Constructor_normalizes_currency_to_uppercase()
    {
        var money = new Money(10m, "usd");

        Assert.Equal("USD", money.Currency);
        Assert.Equal(10m, money.Amount);
    }

    [Fact]
    public void Constructor_trims_currency_whitespace()
    {
        var money = new Money(10m, "  eur  ");

        Assert.Equal("EUR", money.Currency);
    }

    [Fact]
    public void Constructor_rejects_null_currency()
    {
        Assert.Throws<ArgumentNullException>(() => new Money(10m, null!));
    }

    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    public void Constructor_rejects_empty_currency(string currency)
    {
        Assert.Throws<ArgumentException>(() => new Money(10m, currency));
    }

    [Fact]
    public void Zero_uses_usd_by_default()
    {
        var zero = Money.Zero();

        Assert.Equal(0m, zero.Amount);
        Assert.Equal("USD", zero.Currency);
    }

    [Fact]
    public void Add_combines_same_currency_amounts()
    {
        var result = new Money(10m, "USD").Add(new Money(5m, "usd"));

        Assert.Equal(15m, result.Amount);
        Assert.Equal("USD", result.Currency);
    }

    [Fact]
    public void Subtract_combines_same_currency_amounts()
    {
        var result = new Money(10m, "USD") - new Money(4m, "USD");

        Assert.Equal(6m, result.Amount);
    }

    [Fact]
    public void Multiply_scales_amount_and_keeps_currency()
    {
        var result = new Money(10m, "USD") * 2.5m;

        Assert.Equal(25m, result.Amount);
        Assert.Equal("USD", result.Currency);
    }

    [Fact]
    public void Plus_operator_combines_same_currency_amounts()
    {
        var result = new Money(10m, "USD") + new Money(5m, "USD");

        Assert.Equal(15m, result.Amount);
    }

    [Fact]
    public void Add_rejects_cross_currency_arithmetic()
    {
        var left = new Money(10m, "USD");
        var right = new Money(5m, "EUR");

        var add = Assert.Throws<InvalidOperationException>(() => left.Add(right));
        Assert.Contains("USD", add.Message, StringComparison.Ordinal);
        Assert.Contains("EUR", add.Message, StringComparison.Ordinal);

        Assert.Throws<InvalidOperationException>(() => left.Subtract(right));
        Assert.Throws<InvalidOperationException>(() => left + right);
        Assert.Throws<InvalidOperationException>(() => left - right);
    }

    [Fact]
    public void Arithmetic_rejects_null_operands()
    {
        var money = new Money(10m, "USD");

        Assert.Throws<ArgumentNullException>(() => money.Add(null!));
        Assert.Throws<ArgumentNullException>(() => money.Subtract(null!));
        Assert.Throws<ArgumentNullException>(() => Money.Zero() + null!);
        Assert.Throws<ArgumentNullException>(() => Money.Zero() - null!);
        Assert.Throws<ArgumentNullException>(() => Money.Zero(null!).Multiply(2m));
    }
}
