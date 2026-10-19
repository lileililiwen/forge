using System.Linq.Expressions;

namespace Platform.Persistence.EfCore.Specifications;

/// <summary>Represents an application-owned query predicate.</summary>
public abstract class Specification<T>
{
    /// <summary>Creates a specification with its predicate.</summary>
    protected Specification(Expression<Func<T, bool>> criteria)
    {
        Criteria = criteria ?? throw new ArgumentNullException(nameof(criteria));
    }

    /// <summary>Gets the predicate to compose with a consumer query.</summary>
    public Expression<Func<T, bool>> Criteria { get; }
}

/// <summary>Convenient concrete specification for one predicate.</summary>
public sealed class PredicateSpecification<T> : Specification<T>
{
    /// <summary>Creates a predicate specification.</summary>
    public PredicateSpecification(Expression<Func<T, bool>> criteria) : base(criteria) { }
}

/// <summary>Applies a specification without owning the entity or business filter.</summary>
public static class SpecificationEvaluator
{
    /// <summary>Applies the specification predicate to a query.</summary>
    public static IQueryable<T> Where<T>(this IQueryable<T> source, Specification<T> specification)
    {
        ArgumentNullException.ThrowIfNull(source);
        ArgumentNullException.ThrowIfNull(specification);
        return source.Where(specification.Criteria);
    }
}
