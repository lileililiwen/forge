//! Audit event types and a builder.

pub use platform_contracts::payload::audit_event::{
    AuditActor, AuditCategory, AuditEvent, AuditOutcome,
};
pub use platform_contracts::payload::identity_subject::SubjectType as AuditSubjectType;
use platform_core::BoundedString;
use time::OffsetDateTime;

use crate::context::RedactedContext;

/// Builder for [`AuditEvent`].
///
/// The builder exists so producers can construct events in a typed
/// fashion and pass the result to an [`crate::sink::AuditSink`]. All
/// required fields are set in `new`; optional fields are added
/// through `with_*` methods.
pub struct AuditEventBuilder {
    event_id: String,
    occurred_at: OffsetDateTime,
    category: AuditCategory,
    action: BoundedString<128>,
    outcome: AuditOutcome,
    actor: AuditActor,
    tenant_id: String,
    resource: Option<BoundedString<256>>,
    context: RedactedContext,
}

impl AuditEventBuilder {
    /// Construct a builder with the required fields.
    #[must_use]
    pub fn new(
        event_id: impl Into<String>,
        occurred_at: OffsetDateTime,
        category: AuditCategory,
        action: impl Into<String>,
        outcome: AuditOutcome,
        actor_subject_id: impl Into<String>,
        actor_subject_type: AuditSubjectType,
        tenant_id: impl Into<String>,
    ) -> Self {
        let action_raw = action.into();
        let action = BoundedString::try_new(action_raw.clone()).unwrap_or_else(|_| {
            BoundedString::try_new(action_raw.chars().take(128).collect::<String>())
                .expect("clamped")
        });
        Self {
            event_id: event_id.into(),
            occurred_at,
            category,
            action,
            outcome,
            actor: AuditActor {
                subject_id: actor_subject_id.into(),
                subject_type: actor_subject_type,
            },
            tenant_id: tenant_id.into(),
            resource: None,
            context: RedactedContext::new(),
        }
    }

    /// Attach a resource identifier.
    #[must_use]
    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        let raw = resource.into();
        self.resource = BoundedString::try_new(raw.clone())
            .ok()
            .or_else(|| BoundedString::try_new(raw.chars().take(256).collect::<String>()).ok());
        self
    }

    /// Attach a redaction-checked context entry.
    ///
    /// # Errors
    ///
    /// Returns [`crate::context::RedactedContextError`] when the key
    /// matches a secret substring.
    pub fn with_context(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, crate::context::RedactedContextError> {
        self.context.insert(key, value)?;
        Ok(self)
    }

    /// Build the [`AuditEvent`].
    #[must_use]
    pub fn build(self) -> AuditEvent {
        let details = if self.context.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(
                self.context
                    .as_map()
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                    .collect(),
            ))
        };
        AuditEvent {
            event_id: self.event_id,
            occurred_at: format_rfc3339(self.occurred_at),
            category: self.category,
            action: self.action.into_string(),
            outcome: self.outcome,
            actor: self.actor,
            tenant_id: self.tenant_id,
            resource: self.resource.map(BoundedString::into_string),
            details,
        }
    }
}

fn format_rfc3339(instant: OffsetDateTime) -> String {
    instant
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

// Re-export the `Id` type so downstream code that wants to wrap
// identifiers can do so without depending on `platform-core`.
pub use platform_core::id::Id as AuditId;
