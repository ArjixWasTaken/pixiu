//! Email: account links (confirming an address, resetting a password,
//! registration news) and alerts, sent through the mail server admins set
//! in the player.
//!
//! Emails queue in an outbox and go out in the background, a few tries
//! each, so no request waits on (or reveals anything through) the mail
//! server; only the test email is sent while its admin waits.

pub mod templates;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::MultiPart,
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
        extension::ClientId,
    },
};
use tokio::sync::mpsc;

use crate::settings::{Security, Settings, Smtp};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// An email to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email {
    pub to: String,
    pub subject: String,
    pub text: String,
    pub html: String,
}

/// How email leaves: the mail server in production, memory in tests.
pub trait MailTransport: Send + Sync {
    /// Sends `email` through `smtp`, greeting it as `hello` (the public
    /// address's host) when known.
    fn send<'a>(
        &'a self,
        smtp: &'a Smtp,
        hello: Option<&'a str>,
        email: &'a Email,
    ) -> BoxFuture<'a, Result<(), String>>;
}

/// Sends through the mail server over SMTP.
pub struct SmtpTransport;

impl MailTransport for SmtpTransport {
    fn send<'a>(
        &'a self,
        smtp: &'a Smtp,
        hello: Option<&'a str>,
        email: &'a Email,
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            let message = Message::builder()
                .from(
                    smtp.from
                        .parse()
                        .map_err(|error| format!("the sender: {error}"))?,
                )
                .to(email
                    .to
                    .parse()
                    .map_err(|error| format!("the recipient: {error}"))?)
                .subject(&email.subject)
                .multipart(MultiPart::alternative_plain_html(
                    email.text.clone(),
                    email.html.clone(),
                ))
                .map_err(|error| error.to_string())?;
            let tls =
                |host: &str| TlsParameters::new(host.to_owned()).map_err(|error| error.to_string());
            let mut transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp.host)
                .port(smtp.port)
                .timeout(Some(Duration::from_secs(30)))
                .tls(match smtp.security {
                    Security::None => Tls::None,
                    Security::StartTls => Tls::Required(tls(&smtp.host)?),
                    Security::Tls => Tls::Wrapper(tls(&smtp.host)?),
                });
            if let Some(hello) = hello {
                transport = transport.hello_name(ClientId::Domain(hello.to_owned()));
            }
            if let (Some(username), Some(password)) = (&smtp.username, &smtp.password) {
                transport =
                    transport.credentials(Credentials::new(username.clone(), password.clone()));
            }
            transport
                .build()
                .send(message)
                .await
                .map(|_| ())
                .map_err(|error| error.to_string())
        })
    }
}

/// Keeps emails instead of sending them, for tests.
#[derive(Clone, Default)]
pub struct MemoryTransport {
    sent: Arc<Mutex<Vec<Email>>>,
}

impl MemoryTransport {
    /// What was sent so far.
    #[must_use]
    pub fn sent(&self) -> Vec<Email> {
        self.sent.lock().unwrap().clone()
    }
}

impl MailTransport for MemoryTransport {
    fn send<'a>(
        &'a self,
        _smtp: &'a Smtp,
        _hello: Option<&'a str>,
        email: &'a Email,
    ) -> BoxFuture<'a, Result<(), String>> {
        self.sent.lock().unwrap().push(email.clone());
        Box::pin(async { Ok(()) })
    }
}

/// How often an email is tried before it is given up on.
const ATTEMPTS: u32 = 3;

pub struct Mailer {
    settings: Arc<Settings>,
    transport: Arc<dyn MailTransport>,
    outbox: mpsc::UnboundedSender<Email>,
    queued: Mutex<Option<mpsc::UnboundedReceiver<Email>>>,
}

impl Mailer {
    #[must_use]
    pub fn new(settings: Arc<Settings>, transport: Arc<dyn MailTransport>) -> Arc<Self> {
        let (outbox, queued) = mpsc::unbounded_channel();
        Arc::new(Self {
            settings,
            transport,
            outbox,
            queued: Mutex::new(Some(queued)),
        })
    }

    /// Whether email with links in it can be sent.
    #[must_use]
    pub fn ready(&self) -> bool {
        self.settings.get().mail_ready()
    }

    /// The server's settings.
    #[must_use]
    pub fn settings(&self) -> &Arc<Settings> {
        &self.settings
    }

    /// Queues an email; it goes out in the background.
    pub fn queue(&self, email: Email) {
        let _ = self.outbox.send(email);
    }

    /// Sends an email now, for its sender to see how it went.
    ///
    /// # Errors
    ///
    /// Fails, with the mail server's answer, when it cannot be sent.
    pub async fn send_now(&self, email: &Email) -> Result<(), String> {
        let settings = self.settings.get();
        let Some(smtp) = &settings.smtp else {
            return Err("No mail server is set up.".to_owned());
        };
        self.transport.send(smtp, settings.host(), email).await
    }

    /// Sends queued email, until the process ends. Starting twice does
    /// nothing.
    pub fn start(self: &Arc<Self>) {
        let Some(mut queued) = self.queued.lock().unwrap().take() else {
            return;
        };
        let mailer = Arc::clone(self);
        tokio::spawn(async move {
            while let Some(email) = queued.recv().await {
                mailer.deliver(&email).await;
            }
        });
    }

    async fn deliver(&self, email: &Email) {
        for attempt in 1..=ATTEMPTS {
            match self.send_now(email).await {
                Ok(()) => {
                    tracing::info!(subject = %email.subject, "email sent");
                    return;
                }
                Err(error) if attempt < ATTEMPTS => {
                    tracing::warn!(%error, attempt, "cannot send an email; trying again");
                    tokio::time::sleep(Duration::from_secs(10 * u64::from(attempt))).await;
                }
                Err(error) => {
                    tracing::error!(%error, subject = %email.subject, "gave up sending an email");
                }
            }
        }
    }
}
