//! Provider-neutral message model: addresses, messages, bodies.
//!
//! Validation limits (all enforced in [`EmailMessage::validate`], which
//! every [`crate::MailTransport`] implementation MUST call before
//! touching the network):
//!
//! * address: `display-name <local@domain>` or bare `local@domain`;
//!   rejects empty locals, missing `@`, control characters, and
//!   inputs over 254 bytes total.
//! * recipients: `to` is 1..=50; `to + cc + bcc` is at most 50.
//! * subject: at most 998 characters, no CR or LF.
//! * body: text and/or html, at most 5 MiB of declared text.

use crate::error::MailingError;

/// Maximum total address length in bytes (RFC 5321 §4.5.3.1.3).
pub const MAX_ADDRESS_BYTES: usize = 254;

/// Maximum recipient count across `to + cc + bcc`.
pub const MAX_RECIPIENTS: usize = 50;

/// Maximum subject length in characters.
pub const MAX_SUBJECT_CHARS: usize = 998;

/// Maximum declared body size in bytes (5 MiB).
pub const MAX_BODY_BYTES: usize = 5 * 1024 * 1024;

/// A parsed email address: `display-name <local@domain>` or bare
/// `local@domain`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailAddress {
    raw: String,
    local: String,
    domain: String,
    display_name: Option<String>,
}

impl EmailAddress {
    /// Parse and validate an address.
    ///
    /// # Errors
    ///
    /// Returns [`MailingError::Validation`] when the input is empty,
    /// exceeds 254 bytes, contains control characters, has no `@`,
    /// or has an empty local or domain part.
    pub fn parse(input: &str) -> Result<Self, MailingError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(MailingError::validation("email address is empty"));
        }
        if trimmed.len() > MAX_ADDRESS_BYTES {
            return Err(MailingError::validation("email address exceeds 254 bytes"));
        }
        if trimmed.chars().any(|c| c.is_control()) {
            return Err(MailingError::validation(
                "email address contains control characters",
            ));
        }
        let (display_name, addr) = split_display_name(trimmed);
        let (local, domain) = split_addr(addr)?;
        Ok(Self {
            raw: trimmed.to_string(),
            local,
            domain,
            display_name,
        })
    }

    /// The full input as given (trimmed).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// The domain part (safe to log; the local part never is).
    #[must_use]
    pub fn domain(&self) -> &str {
        &self.domain
    }

    /// The optional display name, if `display-name <addr>` form was used.
    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// Redacted form for logs and error text: `...@domain`.
    #[must_use]
    pub fn redacted(&self) -> String {
        format!("...@{}", self.domain)
    }

    /// Bare `local@domain` without any display name.
    #[must_use]
    pub fn bare(&self) -> String {
        format!("{}@{}", self.local, self.domain)
    }
}

/// Split `Display Name <addr>` into its parts; a bare input has no
/// display name.
fn split_display_name(trimmed: &str) -> (Option<String>, &str) {
    if let Some(open) = trimmed.rfind('<') {
        if let Some(close) = trimmed[open..].find('>') {
            let addr = trimmed[open + 1..open + close].trim();
            let name = trimmed[..open].trim().trim_matches('"').trim();
            let name = if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            };
            return (name, addr);
        }
    }
    (None, trimmed)
}

/// Split `local@domain`, rejecting empty parts, spaces, and a missing
/// `@`. The returned strings are owned so the caller never re-slices
/// unvalidated input.
fn split_addr(addr: &str) -> Result<(String, String), MailingError> {
    let Some(at) = addr.rfind('@') else {
        return Err(MailingError::validation(
            "email address has no '@' separator",
        ));
    };
    let (local, rest) = addr.split_at(at);
    let domain = &rest[1..];
    if local.is_empty() {
        return Err(MailingError::validation(
            "email address has an empty local part",
        ));
    }
    if domain.is_empty() {
        return Err(MailingError::validation(
            "email address has an empty domain part",
        ));
    }
    if addr.contains(' ') || addr.contains('\t') {
        return Err(MailingError::validation(
            "email address contains whitespace",
        ));
    }
    Ok((local.to_string(), domain.to_string()))
}

/// Message body: text and/or html. At least one side MUST be present;
/// the declared text MUST NOT exceed 5 MiB in total.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Body {
    text: Option<String>,
    html: Option<String>,
}

impl Body {
    /// Text-only body.
    #[must_use]
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            text: Some(content.into()),
            html: None,
        }
    }

    /// HTML-only body.
    #[must_use]
    pub fn html(content: impl Into<String>) -> Self {
        Self {
            text: None,
            html: Some(content.into()),
        }
    }

    /// Multipart alternative body.
    #[must_use]
    pub fn alternative(text: impl Into<String>, html: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            html: Some(html.into()),
        }
    }

    /// Borrow the text side.
    #[must_use]
    pub fn text_part(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// Borrow the html side.
    #[must_use]
    pub fn html_part(&self) -> Option<&str> {
        self.html.as_deref()
    }

    /// Declared size in bytes across both sides.
    #[must_use]
    pub fn declared_bytes(&self) -> usize {
        self.text.as_ref().map_or(0, String::len) + self.html.as_ref().map_or(0, String::len)
    }

    fn validate(&self) -> Result<(), MailingError> {
        if self.text.is_none() && self.html.is_none() {
            return Err(MailingError::validation(
                "message body is empty: provide text and/or html",
            ));
        }
        if self.declared_bytes() > MAX_BODY_BYTES {
            return Err(MailingError::validation(
                "message body exceeds the 5 MiB declared-size limit",
            ));
        }
        Ok(())
    }
}

/// An attachment carried by reference: the caller loads the bytes; the
/// platform only carries them. No validation of content happens here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentRef {
    filename: String,
    content_type: Option<String>,
    bytes: Vec<u8>,
}

impl AttachmentRef {
    /// Attach already-loaded bytes under a filename.
    ///
    /// # Errors
    ///
    /// Returns [`MailingError::Validation`] when the filename is empty
    /// or contains control characters or path separators.
    pub fn new(
        filename: impl Into<String>,
        content_type: Option<String>,
        bytes: Vec<u8>,
    ) -> Result<Self, MailingError> {
        let filename = filename.into();
        if filename.is_empty() {
            return Err(MailingError::validation("attachment filename is empty"));
        }
        if filename.chars().any(|c| c.is_control()) {
            return Err(MailingError::validation(
                "attachment filename contains control characters",
            ));
        }
        if filename.contains('/') || filename.contains('\\') {
            return Err(MailingError::validation(
                "attachment filename must not contain path separators",
            ));
        }
        Ok(Self {
            filename,
            content_type,
            bytes,
        })
    }

    /// Borrow the filename.
    #[must_use]
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Borrow the declared content type, if any.
    #[must_use]
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// Borrow the caller-loaded bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// A validated-or-validatable outgoing message.
#[derive(Debug, Clone)]
pub struct EmailMessage {
    from: EmailAddress,
    to: Vec<EmailAddress>,
    cc: Vec<EmailAddress>,
    bcc: Vec<EmailAddress>,
    subject: String,
    body: Body,
    attachments: Vec<AttachmentRef>,
}

impl EmailMessage {
    /// Start building a message from its sender.
    #[must_use]
    pub fn builder(from: EmailAddress) -> MessageBuilder {
        MessageBuilder {
            from,
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: String::new(),
            body: None,
            attachments: Vec::new(),
        }
    }

    /// Borrow the sender.
    #[must_use]
    pub fn from(&self) -> &EmailAddress {
        &self.from
    }

    /// Borrow the `to` recipients.
    #[must_use]
    pub fn to(&self) -> &[EmailAddress] {
        &self.to
    }

    /// Borrow the `cc` recipients.
    #[must_use]
    pub fn cc(&self) -> &[EmailAddress] {
        &self.cc
    }

    /// Borrow the `bcc` recipients.
    #[must_use]
    pub fn bcc(&self) -> &[EmailAddress] {
        &self.bcc
    }

    /// Borrow the subject.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Borrow the body.
    #[must_use]
    pub fn body(&self) -> &Body {
        &self.body
    }

    /// Borrow the attachments.
    #[must_use]
    pub fn attachments(&self) -> &[AttachmentRef] {
        &self.attachments
    }

    /// Total recipient count across `to + cc + bcc`.
    #[must_use]
    pub fn recipient_count(&self) -> usize {
        self.to.len() + self.cc.len() + self.bcc.len()
    }

    /// Check every invariant; transports MUST call this before sending.
    ///
    /// # Errors
    ///
    /// Returns [`MailingError::Validation`] for an empty `to` list,
    /// more than 50 total recipients, an over-long or CR/LF-bearing
    /// subject, or an empty/oversize body.
    pub fn validate(&self) -> Result<(), MailingError> {
        if self.to.is_empty() {
            return Err(MailingError::validation("message has no 'to' recipients"));
        }
        if self.recipient_count() > MAX_RECIPIENTS {
            return Err(MailingError::validation(
                "message exceeds the 50-recipient limit",
            ));
        }
        if self.subject.chars().count() > MAX_SUBJECT_CHARS {
            return Err(MailingError::validation("subject exceeds 998 characters"));
        }
        if self.subject.contains('\r') || self.subject.contains('\n') {
            return Err(MailingError::validation(
                "subject must not contain CR or LF",
            ));
        }
        self.body.validate()?;
        Ok(())
    }
}

/// Fluent builder for [`EmailMessage`]; [`build`](Self::build)
/// enforces every invariant.
#[derive(Debug)]
pub struct MessageBuilder {
    from: EmailAddress,
    to: Vec<EmailAddress>,
    cc: Vec<EmailAddress>,
    bcc: Vec<EmailAddress>,
    subject: String,
    body: Option<Body>,
    attachments: Vec<AttachmentRef>,
}

impl MessageBuilder {
    /// Add a `to` recipient.
    #[must_use]
    pub fn to(mut self, addr: EmailAddress) -> Self {
        self.to.push(addr);
        self
    }

    /// Add several `to` recipients.
    #[must_use]
    pub fn extend_to(mut self, addrs: impl IntoIterator<Item = EmailAddress>) -> Self {
        self.to.extend(addrs);
        self
    }

    /// Add a `cc` recipient.
    #[must_use]
    pub fn cc(mut self, addr: EmailAddress) -> Self {
        self.cc.push(addr);
        self
    }

    /// Add a `bcc` recipient.
    #[must_use]
    pub fn bcc(mut self, addr: EmailAddress) -> Self {
        self.bcc.push(addr);
        self
    }

    /// Set the subject.
    #[must_use]
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    /// Set the body.
    #[must_use]
    pub fn body(mut self, body: Body) -> Self {
        self.body = Some(body);
        self
    }

    /// Attach a caller-loaded file reference.
    #[must_use]
    pub fn attachment(mut self, attachment: AttachmentRef) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Build the message, enforcing every invariant.
    ///
    /// # Errors
    ///
    /// Returns [`MailingError::Validation`] for the same reasons as
    /// [`EmailMessage::validate`], plus a missing body.
    pub fn build(self) -> Result<EmailMessage, MailingError> {
        let Some(body) = self.body else {
            return Err(MailingError::validation("message has no body"));
        };
        let message = EmailMessage {
            from: self.from,
            to: self.to,
            cc: self.cc,
            bcc: self.bcc,
            subject: self.subject,
            body,
            attachments: self.attachments,
        };
        message.validate()?;
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(s: &str) -> EmailAddress {
        EmailAddress::parse(s).expect("test address parses")
    }

    fn message(to_count: usize) -> EmailMessage {
        let mut builder = EmailMessage::builder(addr("ops@example.com"))
            .subject("hello")
            .body(Body::text("hi"));
        for i in 0..to_count {
            builder = builder.to(addr(&format!("user{i}@example.com")));
        }
        builder.build().expect("test message builds")
    }

    #[test]
    fn parses_bare_and_display_forms() {
        let bare = addr("alice@example.com");
        assert_eq!(bare.display_name(), None);
        let named = addr("Alice <alice@example.com>");
        assert_eq!(named.display_name(), Some("Alice"));
        assert_eq!(named.bare(), "alice@example.com");
        assert_eq!(named.redacted(), "...@example.com");
    }

    #[test]
    fn rejects_malformed_addresses() {
        for bad in [
            "",
            "   ",
            "no-at-sign",
            "<@example.com>",
            "@example.com",
            "alice@",
            "alice@exam ple.com",
        ] {
            assert!(
                EmailAddress::parse(bad).is_err(),
                "expected rejection: {bad:?}"
            );
        }
    }

    #[test]
    fn rejects_control_chars_and_oversize() {
        assert!(EmailAddress::parse("ali\nce@example.com").is_err());
        let long_local = "a".repeat(250);
        let oversize = format!("{long_local}@example.com");
        assert!(oversize.len() > MAX_ADDRESS_BYTES);
        assert!(EmailAddress::parse(&oversize).is_err());
        // Exactly at the boundary parses.
        let local = "a".repeat(242);
        let boundary = format!("{local}@example.com");
        assert_eq!(boundary.len(), MAX_ADDRESS_BYTES);
        assert!(EmailAddress::parse(&boundary).is_ok());
    }

    #[test]
    fn rejects_empty_to_and_51st_recipient() {
        let no_to = EmailMessage::builder(addr("ops@example.com"))
            .subject("s")
            .body(Body::text("b"))
            .build();
        assert!(matches!(no_to, Err(MailingError::Validation(_))));
        // 50 total recipients is accepted.
        let fifty = message(50);
        assert_eq!(fifty.recipient_count(), 50);
        // The 51st recipient is rejected.
        let too_many = EmailMessage::builder(addr("ops@example.com"))
            .subject("s")
            .body(Body::text("b"))
            .extend_to((0..51).map(|i| addr(&format!("u{i}@example.com"))))
            .build();
        assert!(matches!(too_many, Err(MailingError::Validation(_))));
    }

    #[test]
    fn rejects_crlf_and_oversize_subject() {
        for subject in ["line1\nline2", "line1\rline2"] {
            let result = EmailMessage::builder(addr("ops@example.com"))
                .to(addr("a@example.com"))
                .subject(subject)
                .body(Body::text("b"))
                .build();
            assert!(result.is_err(), "CR/LF subject accepted: {subject:?}");
        }
        let long = "s".repeat(MAX_SUBJECT_CHARS + 1);
        assert!(EmailMessage::builder(addr("ops@example.com"))
            .to(addr("a@example.com"))
            .subject(long)
            .body(Body::text("b"))
            .build()
            .is_err());
    }

    #[test]
    fn rejects_empty_and_oversize_body() {
        let no_body = EmailMessage::builder(addr("ops@example.com"))
            .to(addr("a@example.com"))
            .subject("s")
            .build();
        assert!(no_body.is_err());
        let big = "b".repeat(MAX_BODY_BYTES + 1);
        assert!(EmailMessage::builder(addr("ops@example.com"))
            .to(addr("a@example.com"))
            .subject("s")
            .body(Body::text(big))
            .build()
            .is_err());
        // Exactly 5 MiB is accepted.
        let boundary = "b".repeat(MAX_BODY_BYTES);
        assert!(EmailMessage::builder(addr("ops@example.com"))
            .to(addr("a@example.com"))
            .subject("s")
            .body(Body::text(boundary))
            .build()
            .is_ok());
    }

    #[test]
    fn attachment_filenames_are_constrained() {
        assert!(AttachmentRef::new("", None, vec![]).is_err());
        assert!(AttachmentRef::new("../secret", None, vec![]).is_err());
        assert!(AttachmentRef::new("a\\b", None, vec![]).is_err());
        let ok = AttachmentRef::new(
            "invoice.pdf",
            Some("application/pdf".to_string()),
            vec![1, 2, 3],
        )
        .expect("valid attachment builds");
        assert_eq!(ok.bytes(), &[1, 2, 3]);
    }
}
