//! Opt-in lettre SMTP adapter (feature `smtp-lettre`).
//!
//! [`SmtpTransportConfig`] carries host, port, TLS mode, optional
//! SMTP credentials (wrapped in [`Secret`](platform_core::Secret)),
//! and a connection/send timeout. [`LettreSmtpTransport`] implements
//! [`MailTransport`](crate::MailTransport) over
//! `AsyncSmtpTransport<Tokio1Executor>` using rustls only —
//! native-tls is banned so the transitive tree stays OpenSSL-free.
//!
//! # Error mapping
//!
//! `lettre::transport::smtp::Error` exposes predicates, not a public
//! error-kind enum (its `Kind` is `pub(crate)`), so the total mapping
//! is factored through [`SmtpFailureClass`]: [`classify`] reads the
//! predicates and [`from_class`] maps every class to a
//! [`MailingError`] with an exhaustive `match` (no wildcard arm).
//! [`SmtpFailureClass::ALL`] lets the total-mapping test enumerate
//! every class without constructing lettre errors, whose
//! constructors are crate-private.

use std::time::Duration;

use async_trait::async_trait;
use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use platform_core::Secret;

use crate::error::{scrub_addresses, MailingError};
use crate::transport::{DeliveryReceipt, MailTransport, SendContext};
use crate::types::{EmailAddress, EmailMessage};

/// Default SMTP submission port.
pub const DEFAULT_SMTP_PORT: u16 = 587;

/// Default connect/send timeout.
pub const DEFAULT_SMTP_TIMEOUT: Duration = Duration::from_secs(30);

/// TLS mode for the SMTP connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Security {
    /// Plaintext TCP upgraded with mandatory STARTTLS. The default:
    /// standard submission without silent plaintext fallback.
    #[default]
    StartTlsOrUpgrade,
    /// Implicit TLS from the first byte (SMTPS, usually port 465).
    Tls,
    /// Plaintext only. Accepted ONLY for loopback hosts at
    /// construction; anything else is [`MailingError::Configuration`].
    None,
}

/// Adapter configuration. Credentials stay inside
/// [`Secret`](platform_core::Secret) so `Debug` on this type cannot
/// leak them.
#[derive(Debug, Clone)]
pub struct SmtpTransportConfig {
    host: String,
    port: u16,
    security: Security,
    username: Option<Secret<String>>,
    password: Option<Secret<String>>,
    timeout: Duration,
}

impl SmtpTransportConfig {
    /// Build a config for `host` with the given TLS mode, default
    /// port 587, no credentials, and the default 30 s timeout.
    ///
    /// # Errors
    ///
    /// Returns [`MailingError::Configuration`] for an empty host or
    /// for [`Security::None`] with a non-loopback host.
    pub fn new(host: impl Into<String>, security: Security) -> Result<Self, MailingError> {
        let host = host.into();
        if host.trim().is_empty() {
            return Err(MailingError::configuration("smtp host is empty"));
        }
        if security == Security::None && !is_loopback_host(&host) {
            return Err(MailingError::configuration(
                "insecure smtp (Security::None) is refused off-loopback",
            ));
        }
        Ok(Self {
            host,
            port: DEFAULT_SMTP_PORT,
            security,
            username: None,
            password: None,
            timeout: DEFAULT_SMTP_TIMEOUT,
        })
    }

    /// Override the port.
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set SMTP AUTH credentials.
    #[must_use]
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.username = Some(Secret::new(username.into()));
        self.password = Some(Secret::new(password.into()));
        self
    }

    /// Override the connect/send timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Borrow the host.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// The port.
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// The TLS mode.
    #[must_use]
    pub const fn security(&self) -> Security {
        self.security
    }

    /// The timeout.
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Whether AUTH credentials are configured (never the values).
    #[must_use]
    pub const fn has_credentials(&self) -> bool {
        self.username.is_some() && self.password.is_some()
    }
}

/// Loopback rule for [`Security::None`]: `localhost`, `127.0.0.0/8`,
/// and `::1` (bare or bracketed). Matching is case-insensitive for
/// the name; anything else is a remote host.
#[must_use]
pub fn is_loopback_host(host: &str) -> bool {
    let lower = host.trim().to_ascii_lowercase();
    lower == "localhost"
        || lower == "::1"
        || lower == "[::1]"
        || lower == "127.0.0.1"
        || lower.starts_with("127.")
}

/// The lettre SMTP transport. Holds the async client; every `send`
/// re-validates the message, renders it into a lettre [`Message`],
/// and maps the lettre error onto [`MailingError`].
pub struct LettreSmtpTransport {
    config: SmtpTransportConfig,
    inner: AsyncSmtpTransport<Tokio1Executor>,
}

impl LettreSmtpTransport {
    /// Build the transport from its config. No network I/O happens
    /// here; connection errors surface at `send` time.
    ///
    /// # Errors
    ///
    /// Returns [`MailingError::Configuration`] when TLS parameters
    /// cannot be built for the configured host.
    pub fn new(config: SmtpTransportConfig) -> Result<Self, MailingError> {
        let tls = match config.security {
            Security::None => Tls::None,
            Security::StartTlsOrUpgrade => {
                let params = TlsParameters::new(config.host.clone()).map_err(|_| {
                    MailingError::configuration("could not build TLS parameters for the smtp host")
                })?;
                Tls::Required(params)
            }
            Security::Tls => {
                let params = TlsParameters::new(config.host.clone()).map_err(|_| {
                    MailingError::configuration("could not build TLS parameters for the smtp host")
                })?;
                Tls::Wrapper(params)
            }
        };
        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
            .port(config.port)
            .timeout(Some(config.timeout))
            .tls(tls);
        if let (Some(user), Some(pass)) = (config.username.as_ref(), config.password.as_ref()) {
            builder = builder.credentials(Credentials::new(
                user.expose().clone(),
                pass.expose().clone(),
            ));
        }
        Ok(Self {
            config,
            inner: builder.build(),
        })
    }

    /// Borrow the config.
    #[must_use]
    pub fn config(&self) -> &SmtpTransportConfig {
        &self.config
    }

    /// Render a validated message into a lettre [`Message`]. `Bcc`
    /// recipients enter the envelope and are stripped from the wire
    /// headers by lettre's default `drop_bcc` behavior.
    fn render(msg: &EmailMessage) -> Result<Message, MailingError> {
        let mut builder = Message::builder().from(to_mailbox(msg.from())?);
        for addr in msg.to() {
            builder = builder.to(to_mailbox(addr)?);
        }
        for addr in msg.cc() {
            builder = builder.cc(to_mailbox(addr)?);
        }
        for addr in msg.bcc() {
            builder = builder.bcc(to_mailbox(addr)?);
        }
        // The subject was validated CR/LF-free, so lettre cannot
        // reject it; a failure here maps to validation regardless.
        builder = builder.subject(msg.subject());
        let body = msg.body();
        let rendered = match (body.text_part(), body.html_part()) {
            (Some(text), Some(html)) => builder.multipart(MultiPart::alternative_plain_html(
                text.to_string(),
                html.to_string(),
            )),
            (Some(text), None) => builder.body(text.to_string()),
            (None, Some(html)) => builder
                .multipart(MultiPart::alternative().singlepart(SinglePart::html(html.to_string()))),
            (None, None) => {
                return Err(MailingError::validation("message has no body"));
            }
        };
        rendered.map_err(|_| MailingError::validation("message could not be rendered for sending"))
    }
}

/// Convert our address into a lettre [`Mailbox`]. Our parser already
/// validated the shape; a lettre rejection maps to validation with
/// the domain only (never the local part).
fn to_mailbox(addr: &EmailAddress) -> Result<Mailbox, MailingError> {
    let bare = addr.bare();
    bare.parse::<Mailbox>().map_err(|_| {
        MailingError::validation(format!(
            "address for domain {} is not sendable",
            addr.domain()
        ))
    })
}

/// Failure classes of the SMTP send path. Mirrors the predicate set
/// on `lettre::transport::smtp::Error` (`is_timeout`, `is_tls`,
/// `is_transient`/`is_permanent` + `status`, client, and the
/// connection/network/response/shutdown remainder), whose `Kind`
/// enum is crate-private and cannot be matched externally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtpFailureClass {
    /// The send exceeded the configured timeout.
    Timeout,
    /// TLS negotiation failed.
    Tls,
    /// The server rejected the credentials (5xx auth, e.g. 535).
    Authentication,
    /// The server permanently rejected the message (other 5xx).
    RejectedByServer,
    /// The path is down: DNS, refused, reset, 4xx transient,
    /// unparsable response, or shut-down transport.
    TransportUnavailable,
    /// The client was misconfigured (lettre internal client error).
    Configuration,
}

impl SmtpFailureClass {
    /// Every class, for the total-mapping test. Adding a variant
    /// without extending this table AND the `from_class` match fails
    /// the test (and the non-wildcard match fails to compile).
    pub const ALL: &'static [Self] = &[
        Self::Timeout,
        Self::Tls,
        Self::Authentication,
        Self::RejectedByServer,
        Self::TransportUnavailable,
        Self::Configuration,
    ];
}

/// Classify a lettre SMTP error via its public predicates. Timeout
/// wins over everything (a TLS handshake can time out); TLS wins
/// over status; a 535 permanent failure is authentication; other
/// permanent failures are rejections; transient/status-less errors
/// are unavailability; internal client errors are configuration.
fn classify(err: &lettre::transport::smtp::Error) -> (SmtpFailureClass, Option<u16>) {
    if err.is_timeout() {
        return (SmtpFailureClass::Timeout, None);
    }
    if err.is_tls() {
        return (SmtpFailureClass::Tls, None);
    }
    let code = err.status().and_then(|c| c.to_string().parse::<u16>().ok());
    if err.is_permanent() {
        if code == Some(535) {
            return (SmtpFailureClass::Authentication, code);
        }
        return (SmtpFailureClass::RejectedByServer, code);
    }
    if err.is_client() {
        return (SmtpFailureClass::Configuration, code);
    }
    (SmtpFailureClass::TransportUnavailable, code)
}

/// Map a classified failure onto [`MailingError`]. The match is
/// exhaustive with no wildcard: a new class breaks the build here.
/// Server-derived text is scrubbed; auth/TLS/timeout carry no server
/// text at all.
fn from_class(
    class: SmtpFailureClass,
    code: Option<u16>,
    detail: &str,
    timeout: Duration,
) -> MailingError {
    match class {
        SmtpFailureClass::Timeout => MailingError::timeout(timeout),
        SmtpFailureClass::Tls => MailingError::Tls,
        SmtpFailureClass::Authentication => MailingError::Authentication,
        SmtpFailureClass::RejectedByServer => MailingError::rejected_by_server(code, detail),
        SmtpFailureClass::TransportUnavailable => MailingError::transport_unavailable(detail),
        SmtpFailureClass::Configuration => {
            // Client errors are internal, but their text can echo
            // address material, so it is scrubbed like server text.
            MailingError::configuration(scrub_addresses(detail))
        }
    }
}

/// Map a lettre SMTP error onto [`MailingError`].
fn map_lettre_error(err: lettre::transport::smtp::Error, timeout: Duration) -> MailingError {
    let raw = err.to_string();
    let (class, code) = classify(&err);
    from_class(class, code, &raw, timeout)
}

#[async_trait]
impl MailTransport for LettreSmtpTransport {
    async fn send(
        &self,
        msg: &EmailMessage,
        cx: &SendContext,
    ) -> Result<DeliveryReceipt, MailingError> {
        // Validation first: an invalid message never renders, let
        // alone reaches the network.
        msg.validate()?;
        let rendered = Self::render(msg)?;
        self.inner
            .send(rendered)
            .await
            .map_err(|err| map_lettre_error(err, self.config.timeout))?;
        // SMTP `send` returns `()` on acceptance: no provider message
        // id exists. The timestamp comes from the injected clock.
        Ok(DeliveryReceipt::new(None, cx.clock().now()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Body;

    fn test_message() -> EmailMessage {
        EmailMessage::builder(EmailAddress::parse("ops@example.com").expect("addr"))
            .to(EmailAddress::parse("alice@example.com").expect("addr"))
            .subject("hello")
            .body(Body::text("hi"))
            .build()
            .expect("message builds")
    }

    #[test]
    fn loopback_rule_accepts_local_and_rejects_remote() {
        for host in [
            "localhost",
            "LOCALHOST",
            "127.0.0.1",
            "127.0.0.2",
            "::1",
            "[::1]",
        ] {
            assert!(is_loopback_host(host), "{host} should be loopback");
            assert!(
                SmtpTransportConfig::new(host, Security::None).is_ok(),
                "{host} with Security::None should construct"
            );
        }
        for host in ["mail.example.com", "192.168.1.1", "10.0.0.1", "example.com"] {
            assert!(!is_loopback_host(host), "{host} should not be loopback");
            assert!(
                matches!(
                    SmtpTransportConfig::new(host, Security::None),
                    Err(MailingError::Configuration(_))
                ),
                "{host} with Security::None must fail"
            );
        }
        // Secure modes accept any host.
        assert!(SmtpTransportConfig::new("mail.example.com", Security::StartTlsOrUpgrade).is_ok());
    }

    #[test]
    fn total_mapping_covers_every_failure_class() {
        // One row per SmtpFailureClass::ALL entry: the table length
        // check forces the test to grow with the enum, and the match
        // in from_class has no wildcard so the compiler enforces the
        // same totality on the implementation side.
        assert_eq!(SmtpFailureClass::ALL.len(), 6);
        for class in SmtpFailureClass::ALL {
            let err = from_class(
                *class,
                Some(550),
                "mailbox unavailable: alice@example.com",
                Duration::from_secs(7),
            );
            let rendered = err.to_string();
            assert!(
                !rendered.contains("alice@"),
                "{class:?} leaked local-part: {rendered}"
            );
            match class {
                SmtpFailureClass::Timeout => {
                    assert!(matches!(err, MailingError::Timeout { .. }));
                }
                SmtpFailureClass::Tls => assert!(matches!(err, MailingError::Tls)),
                SmtpFailureClass::Authentication => {
                    assert!(matches!(err, MailingError::Authentication))
                }
                SmtpFailureClass::RejectedByServer => {
                    assert!(matches!(err, MailingError::RejectedByServer { .. }));
                }
                SmtpFailureClass::TransportUnavailable => {
                    assert!(matches!(err, MailingError::TransportUnavailable { .. }));
                }
                SmtpFailureClass::Configuration => {
                    assert!(matches!(err, MailingError::Configuration(_)))
                }
            }
        }
    }

    #[test]
    fn config_debug_redacts_credentials() {
        let config = SmtpTransportConfig::new("mail.example.com", Security::Tls)
            .expect("config builds")
            .with_credentials("alice", "hunter2");
        let rendered = format!("{config:?}");
        assert!(!rendered.contains("hunter2"), "password leaked");
        assert!(!rendered.contains("alice"), "username leaked");
    }

    #[tokio::test]
    async fn refused_connection_maps_to_unavailable_without_network() {
        // 127.0.0.1 with a (near-certainly) closed port: the
        // connection is refused locally, exercising the real
        // lettre error path with no external network.
        let config = SmtpTransportConfig::new("127.0.0.1", Security::None)
            .expect("loopback plaintext constructs")
            .with_port(1)
            .with_timeout(Duration::from_secs(5));
        let transport = LettreSmtpTransport::new(config).expect("transport builds");
        let clock = platform_core::FixedClock::at_epoch();
        let cx = SendContext::new(platform_core::CorrelationId::new(), &clock);
        let err = transport
            .send(&test_message(), &cx)
            .await
            .expect_err("closed port must fail");
        assert!(
            matches!(err, MailingError::TransportUnavailable { .. }),
            "expected unavailable, got {err}"
        );
    }
}
