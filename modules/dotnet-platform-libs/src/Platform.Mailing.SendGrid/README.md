# Platform.Mailing.SendGrid

Optional SendGrid adapter for the provider-neutral `Platform.Mailing`
contracts, built on the SendGrid SDK client. Depends on `Platform.Mailing`,
the SendGrid client, and the core `Microsoft.Extensions.*` abstractions only;
it owns no templates, retry policies, delivery records, or provider
credentials beyond the configured options.
