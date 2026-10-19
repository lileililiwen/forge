# Design

Define `INotificationSender`, `ISmsSender`, normalized message/value types, delivery outcome,
failure category, template ID, and provider status. Email remains implemented by
`Platform.Mailing`; notification orchestration can select email or SMS through explicit
channels.

Providers are adapters. A console provider is allowed only in development/test and records
messages without pretending delivery succeeded in production. Scheduled notifications use
`Platform.Jobs`; repeated delivery uses `Platform.Idempotency`; domain-triggered notifications
use `Platform.Eventing`.
