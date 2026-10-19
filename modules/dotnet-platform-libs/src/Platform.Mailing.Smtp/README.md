# Platform.Mailing.Smtp

Optional SMTP adapter for the provider-neutral `Platform.Mailing` contracts,
built on MailKit. Depends on `Platform.Mailing`, MailKit, and the core
`Microsoft.Extensions.*` abstractions only; it owns no templates, retry
policies, delivery records, or provider credentials beyond the configured
SMTP options.
