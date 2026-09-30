//! The server's settings that admins change in the player: the address
//! people reach it at, whether anyone may register, the mail server, and
//! the single sign-on provider.
//! They are stored as JSON rows of the `settings` table, secrets sealed
//! with the instance key, and kept in memory for everyone to read.

use std::sync::Arc;

use pixiu_core::SecretBox;
use pixiu_db::{Db, Setting, toasty};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::AccountError;

/// `{public_url, registration_open}`.
const SERVER: &str = "server";
/// The mail server, see [`Smtp`].
const SMTP: &str = "mail.smtp";
/// The single sign-on provider, see [`Oidc`].
const OIDC: &str = "auth.oidc";

/// How the connection to the mail server is secured.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Security {
    /// Plain: only for a mail server on the same host or network.
    #[default]
    None,
    /// Upgraded to TLS after connecting (usually port 587).
    StartTls,
    /// TLS from the start (usually port 465).
    Tls,
}

/// The mail server píxiū sends through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Smtp {
    pub host: String,
    pub port: u16,
    pub security: Security,
    pub username: Option<String>,
    pub password: Option<String>,
    /// The sender, e.g. `píxiū <no-reply@example.com>`.
    pub from: String,
}

/// [`Smtp`] as stored: the password sealed.
#[derive(Serialize, Deserialize)]
struct StoredSmtp {
    host: String,
    port: u16,
    security: Security,
    username: Option<String>,
    password: Option<String>,
    from: String,
}

/// The OpenID Connect provider people may sign in with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Oidc {
    /// What the sign-in button says: "Sign in with <name>".
    pub name: String,
    /// The issuer URL; its discovery document says the rest.
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    /// Asked for besides `openid`.
    pub scopes: Vec<String>,
}

/// [`Oidc`] as stored: the secret sealed.
#[derive(Serialize, Deserialize)]
struct StoredOidc {
    name: String,
    issuer: String,
    client_id: String,
    client_secret: String,
    scopes: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct StoredServer {
    public_url: Option<String>,
    #[serde(default)]
    registration_open: bool,
}

/// The settings as they are now.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServerSettings {
    /// Where people reach píxiū, e.g. `https://music.example.com`; links in
    /// emails start with it.
    pub public_url: Option<String>,
    /// Whether anyone may ask for an account (admins approve each).
    pub registration_open: bool,
    pub smtp: Option<Smtp>,
    pub oidc: Option<Oidc>,
}

impl ServerSettings {
    /// Whether píxiū can send email with links in it.
    #[must_use]
    pub fn mail_ready(&self) -> bool {
        self.smtp.is_some() && self.public_url.is_some()
    }

    /// Whether strangers may register now: it is open, and mail works.
    #[must_use]
    pub fn registration_available(&self) -> bool {
        self.registration_open && self.mail_ready()
    }

    /// Where the single sign-on provider sends people back.
    #[must_use]
    pub fn redirect_uri(&self) -> Option<String> {
        self.link("/api/auth/oidc/callback")
    }

    /// The provider people may sign in with now: set up, with a public
    /// address to come back to.
    #[must_use]
    pub fn sso(&self) -> Option<&Oidc> {
        self.oidc.as_ref().filter(|_| self.public_url.is_some())
    }

    /// A link to `path` (starting with `/`) on the public address.
    #[must_use]
    pub fn link(&self, path: &str) -> Option<String> {
        self.public_url.as_ref().map(|url| format!("{url}{path}"))
    }

    /// The public address's host, for greeting mail servers.
    #[must_use]
    pub fn host(&self) -> Option<&str> {
        let url = self.public_url.as_deref()?;
        let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
        rest.split(['/', ':'])
            .next()
            .filter(|host| !host.is_empty())
    }
}

/// Checks an address people reach píxiū at: `http(s)://host[:port][/path]`,
/// kept without a trailing slash.
///
/// # Errors
///
/// Fails, with a message for people, when it is not one.
pub fn check_public_url(url: &str) -> Result<String, AccountError> {
    let url = url.trim().trim_end_matches('/');
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    match rest {
        Some(rest)
            if !rest.is_empty()
                && !rest.starts_with('/')
                && !rest.chars().any(|c| c.is_whitespace() || c.is_control()) =>
        {
            Ok(url.to_owned())
        }
        _ => Err(AccountError::Invalid(
            "The address starts with https:// (or http://), like https://music.example.com."
                .to_owned(),
        )),
    }
}

async fn read(db: &Db, key: &str) -> Result<Option<String>, toasty::Error> {
    Ok(Setting::filter_by_key(key)
        .first()
        .exec(&mut db.clone())
        .await?
        .map(|setting| setting.value))
}

/// The server's settings, loaded once and kept current.
pub struct Settings {
    db: Db,
    secrets: SecretBox,
    current: watch::Sender<Arc<ServerSettings>>,
}

impl Settings {
    /// Reads the stored settings.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn load(db: Db, secrets: SecretBox) -> Result<Arc<Self>, toasty::Error> {
        let server: StoredServer = read(&db, SERVER)
            .await?
            .and_then(|value| serde_json::from_str(&value).ok())
            .unwrap_or_default();
        let smtp = read(&db, SMTP)
            .await?
            .and_then(|value| serde_json::from_str::<StoredSmtp>(&value).ok())
            .map(|stored| Smtp {
                password: stored.password.and_then(|sealed| {
                    secrets
                        .open_str(&sealed)
                        .inspect_err(|error| {
                            tracing::warn!(%error, "cannot open the stored mail server password");
                        })
                        .ok()
                }),
                host: stored.host,
                port: stored.port,
                security: stored.security,
                username: stored.username,
                from: stored.from,
            });
        let oidc = read(&db, OIDC)
            .await?
            .and_then(|value| serde_json::from_str::<StoredOidc>(&value).ok())
            .and_then(|stored| {
                let client_secret = secrets
                    .open_str(&stored.client_secret)
                    .inspect_err(|error| {
                        tracing::warn!(%error, "cannot open the stored single sign-on secret");
                    })
                    .ok()?;
                Some(Oidc {
                    name: stored.name,
                    issuer: stored.issuer,
                    client_id: stored.client_id,
                    client_secret,
                    scopes: stored.scopes,
                })
            });
        let settings = ServerSettings {
            public_url: server.public_url,
            registration_open: server.registration_open,
            smtp,
            oidc,
        };
        Ok(Arc::new(Self {
            db,
            secrets,
            current: watch::channel(Arc::new(settings)).0,
        }))
    }

    /// The settings as they are now.
    #[must_use]
    pub fn get(&self) -> Arc<ServerSettings> {
        Arc::clone(&self.current.borrow())
    }

    /// Hears about every change.
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<Arc<ServerSettings>> {
        self.current.subscribe()
    }

    async fn store(&self, key: &str, value: String) -> Result<(), toasty::Error> {
        let mut db = self.db.clone();
        match Setting::filter_by_key(key).first().exec(&mut db).await? {
            Some(mut setting) => toasty::update!(setting { value }).exec(&mut db).await,
            None => toasty::create!(Setting { key, value })
                .exec(&mut db)
                .await
                .map(|_| ()),
        }
    }

    async fn store_server(&self, settings: &ServerSettings) -> Result<(), toasty::Error> {
        let stored = StoredServer {
            public_url: settings.public_url.clone(),
            registration_open: settings.registration_open,
        };
        self.store(
            SERVER,
            serde_json::to_string(&stored).expect("settings serialize"),
        )
        .await
    }

    /// Where people reach píxiū; `None` forgets it.
    ///
    /// # Errors
    ///
    /// Fails when it is not an address, or on database errors.
    pub async fn set_public_url(&self, url: Option<&str>) -> Result<(), AccountError> {
        let url = url
            .filter(|url| !url.trim().is_empty())
            .map(check_public_url)
            .transpose()?;
        let mut settings = (*self.get()).clone();
        settings.public_url = url;
        self.store_server(&settings).await?;
        self.current.send_replace(Arc::new(settings));
        Ok(())
    }

    /// Opens or closes registration.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn set_registration_open(&self, open: bool) -> Result<(), AccountError> {
        let mut settings = (*self.get()).clone();
        settings.registration_open = open;
        self.store_server(&settings).await?;
        self.current.send_replace(Arc::new(settings));
        Ok(())
    }

    /// The mail server; `None` forgets it (and with it, email).
    ///
    /// # Errors
    ///
    /// Fails when a value is missing, or on database errors.
    pub async fn set_smtp(&self, smtp: Option<Smtp>) -> Result<(), AccountError> {
        match &smtp {
            Some(smtp) => {
                if smtp.host.trim().is_empty() || smtp.from.trim().is_empty() || smtp.port == 0 {
                    return Err(AccountError::Invalid(
                        "Name the mail server, its port, and who mail comes from.".to_owned(),
                    ));
                }
                let stored = StoredSmtp {
                    host: smtp.host.trim().to_owned(),
                    port: smtp.port,
                    security: smtp.security,
                    username: smtp.username.clone().filter(|name| !name.is_empty()),
                    password: smtp
                        .password
                        .as_deref()
                        .filter(|password| !password.is_empty())
                        .map(|password| self.secrets.seal_str(password)),
                    from: smtp.from.trim().to_owned(),
                };
                self.store(
                    SMTP,
                    serde_json::to_string(&stored).expect("settings serialize"),
                )
                .await?;
            }
            None => {
                Setting::filter_by_key(SMTP)
                    .delete()
                    .exec(&mut self.db.clone())
                    .await?;
            }
        }
        let mut settings = (*self.get()).clone();
        settings.smtp = smtp;
        self.current.send_replace(Arc::new(settings));
        Ok(())
    }
}

impl Settings {
    /// The single sign-on provider; `None` forgets it.
    ///
    /// # Errors
    ///
    /// Fails when a value is missing or the issuer is no URL, or on
    /// database errors.
    pub async fn set_oidc(&self, oidc: Option<Oidc>) -> Result<(), AccountError> {
        match &oidc {
            Some(oidc) => {
                if [&oidc.name, &oidc.client_id, &oidc.client_secret]
                    .iter()
                    .any(|value| value.trim().is_empty())
                {
                    return Err(AccountError::Invalid(
                        "Name the provider, and give its client id and secret.".to_owned(),
                    ));
                }
                check_issuer(&oidc.issuer)?;
                let stored = StoredOidc {
                    name: oidc.name.trim().to_owned(),
                    issuer: oidc.issuer.trim().to_owned(),
                    client_id: oidc.client_id.trim().to_owned(),
                    client_secret: self.secrets.seal_str(&oidc.client_secret),
                    scopes: oidc.scopes.clone(),
                };
                self.store(
                    OIDC,
                    serde_json::to_string(&stored).expect("settings serialize"),
                )
                .await?;
            }
            None => {
                Setting::filter_by_key(OIDC)
                    .delete()
                    .exec(&mut self.db.clone())
                    .await?;
            }
        }
        let mut settings = (*self.get()).clone();
        settings.oidc = oidc.map(|oidc| Oidc {
            name: oidc.name.trim().to_owned(),
            issuer: oidc.issuer.trim().to_owned(),
            client_id: oidc.client_id.trim().to_owned(),
            ..oidc
        });
        self.current.send_replace(Arc::new(settings));
        Ok(())
    }
}

/// Checks an issuer URL: `https://` (or `http://`, for a provider on the
/// same machine), kept as given, since ID tokens must name it exactly.
fn check_issuer(issuer: &str) -> Result<(), AccountError> {
    let issuer = issuer.trim();
    let rest = issuer
        .strip_prefix("https://")
        .or_else(|| issuer.strip_prefix("http://"));
    match rest {
        Some(rest) if !rest.is_empty() && !rest.chars().any(char::is_whitespace) => Ok(()),
        _ => Err(AccountError::Invalid(
            "The issuer is the provider's URL, like https://sso.example.com.".to_owned(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_addresses_are_checked() {
        assert_eq!(
            check_public_url(" https://music.example.com/ ").unwrap(),
            "https://music.example.com"
        );
        assert_eq!(
            check_public_url("http://localhost:4533").unwrap(),
            "http://localhost:4533"
        );
        for bad in [
            "music.example.com",
            "ftp://x",
            "https://",
            "https:///x",
            "https://a b",
        ] {
            assert!(check_public_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn links_start_with_the_public_address() {
        let settings = ServerSettings {
            public_url: Some("https://music.example.com".to_owned()),
            ..ServerSettings::default()
        };
        assert_eq!(
            settings.link("/reset-password/abc").as_deref(),
            Some("https://music.example.com/reset-password/abc")
        );
        assert_eq!(settings.host(), Some("music.example.com"));
        assert!(!settings.mail_ready());
    }
}
