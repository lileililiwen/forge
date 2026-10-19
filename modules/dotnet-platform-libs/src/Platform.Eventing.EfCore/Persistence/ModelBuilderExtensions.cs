using Microsoft.EntityFrameworkCore;

namespace Platform.Eventing.EfCore;

/// <summary>EF Core model configuration for application-owned durable event tables.</summary>
public static class ModelBuilderExtensions
{
    /// <summary>Maps durable event entities into the supplied application model.</summary>
    public static ModelBuilder ConfigurePlatformEventing(this ModelBuilder modelBuilder, PlatformEventingModelOptions? options = null)
    {
        ArgumentNullException.ThrowIfNull(modelBuilder);
        options ??= new PlatformEventingModelOptions();
        options.Validate();

        modelBuilder.Entity<PlatformOutboxMessage>(entity =>
        {
            entity.ToTable(options.OutboxTableName);
            entity.HasKey(message => message.MessageId);
            entity.Property(message => message.MessageId).HasMaxLength(200);
            entity.Property(message => message.PayloadType).HasMaxLength(500).IsRequired();
            entity.Property(message => message.PayloadJson).IsRequired();
            entity.Property(message => message.State).HasConversion<int>();
            entity.HasIndex(message => new { message.State, message.NextAttemptAt, message.LeaseExpiresAt });
        });

        modelBuilder.Entity<PlatformInboxMessage>(entity =>
        {
            entity.ToTable(options.InboxTableName);
            entity.HasKey(message => message.MessageId);
            entity.Property(message => message.MessageId).HasMaxLength(200);
            entity.Property(message => message.PayloadType).HasMaxLength(500).IsRequired();
            entity.Property(message => message.PayloadJson).IsRequired();
            entity.Property(message => message.State).HasConversion<int>();
            entity.HasIndex(message => new { message.State, message.NextAttemptAt, message.LeaseExpiresAt });
        });

        return modelBuilder;
    }
}
