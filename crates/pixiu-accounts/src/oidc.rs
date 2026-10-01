//! Single sign-on through an OpenID Connect provider (Authelia, say).
//!
//! The authorization code flow, with PKCE (S256), `state`, `nonce`, and a
//! cookie binding each sign-in to the browser that started it. People link
//! an account there to theirs here under Settings, then sign in with it;
//! signing in never makes accounts. A sign-in ends with a one-time code the
//! player trades for its token, so no token travels in a URL.

use std::{
    collections::HashMap,
    fmt::Write as _,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use openidconnect::{
    AdditionalProviderMetadata, AuthorizationCode, ClaimsVerificationError, ClientId, ClientSecret,
    CsrfToken, EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, ProviderMetadata, RedirectUrl, Scope, SignatureVerificationError,
    TokenResponse,
    core::{
        CoreAuthDisplay, CoreAuthenticationFlow, CoreClaimName, CoreClaimType, CoreClient,
        CoreClientAuthMethod, CoreGrantType, CoreJsonWebKey, CoreJweContentEncryptionAlgorithm,
        CoreJweKeyManagementAlgorithm, CoreResponseMode, CoreResponseType,
        CoreSubjectIdentifierType,
    },
    reqwest,
};
use pixiu_db::ApiKey;
use serde::{Deserialize, Serialize};

use crate::{Oidc, Settings};

/// How long someone has at the provider.
const PENDING_FOR: Duration = Duration::from_secs(10 * 60);
/// At most this many sign-ins wait at once; the oldest go first.
const MAX_PENDING: usize = 1000;
/// How long the player has to trade a sign-in's code for its token.
const CODE_FOR: Duration = Duration::from_secs(60);
/// Asked for besides `openid` when the admin names no scopes.
const DEFAULT_SCOPES: [&str; 2] = ["profile", "email"];

/// Discovery fields beyond OpenID Connect's: whether PKCE's S256 works.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PkceMetadata {
    #[serde(default)]
    code_challenge_methods_supported: Vec<String>,
}

impl AdditionalProviderMetadata for PkceMetadata {}

type Metadata = ProviderMetadata<
    PkceMetadata,
    CoreAuthDisplay,
    CoreClientAuthMethod,
    CoreClaimName,
    CoreClaimType,
    CoreGrantType,
    CoreJweContentEncryptionAlgorithm,
    CoreJweKeyManagementAlgorithm,
    CoreJsonWebKey,
    CoreResponseMode,
    CoreResponseType,
    CoreSubjectIdentifierType,
>;

/// A client made from discovery: the authorization endpoint is known, the
/// token and user info endpoints may be.
type Client = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

#[derive(Debug, thiserror::Error)]
pub enum SsoError {
    #[error("single sign-on is not set up")]
    Unavailable,
    /// The sign-in is unknown, expired, used, or from another browser.
    #[error("the sign-in expired, or was started in another browser")]
    Expired,
    /// The provider said no (the person cancelled, say).
    #[error("the provider refused: {0}")]
    Refused(String),
    #[error("{0}")]
    Provider(String),
}

/// What a sign-in is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// Signing in.
    Login,
    /// Linking an account at the provider to this user's.
    Link(u64),
}

/// A sign-in waiting for the provider to send its person back.
pub struct Pending {
    intent: Intent,
    nonce: Nonce,
    verifier: PkceCodeVerifier,
    /// The hash of the cookie the starting browser got.
    binding: String,
    started: Instant,
}

impl Pending {
    #[must_use]
    pub fn intent(&self) -> Intent {
        self.intent
    }
}

/// A sign-in on its way to the provider.
pub struct Started {
    /// Where to send the browser.
    pub url: String,
    /// For the binding cookie.
    pub binding: String,
}

/// Who the provider says signed in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub issuer: String,
    pub subject: String,
    pub email: Option<String>,
}

/// What testing a provider found.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// Its signing keys.
    pub keys: usize,
    /// Whether it takes PKCE's S256, as píxiū uses.
    pub pkce_s256: bool,
}

struct Provider {
    oidc: Oidc,
    redirect: String,
    client: Client,
    report: Report,
}

/// The single sign-on provider, as set up in the settings.
pub struct Sso {
    settings: Arc<Settings>,
    http: reqwest::Client,
    /// Discovered once per setup, and again when its keys rotate.
    provider: tokio::sync::Mutex<Option<Arc<Provider>>>,
    /// By `state`.
    pending: Mutex<HashMap<String, Pending>>,
    /// By the code's hash: the user and when it was made.
    codes: Mutex<HashMap<String, (u64, Instant)>>,
}

fn random_token() -> String {
    URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>())
}

/// An error and its causes, which is where the useful part usually is.
fn explain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        let _ = write!(text, ": {cause}");
        source = cause.source();
    }
    text
}

async fn discover(
    http: &reqwest::Client,
    oidc: &Oidc,
    redirect: &str,
) -> Result<Provider, SsoError> {
    let issuer = IssuerUrl::new(oidc.issuer.clone())
        .map_err(|error| SsoError::Provider(format!("the issuer is no URL: {error}")))?;
    let metadata = Metadata::discover_async(issuer, http)
        .await
        .map_err(|error| SsoError::Provider(format!("discovery failed: {}", explain(&error))))?;
    let report = Report {
        keys: metadata.jwks().keys().len(),
        pkce_s256: metadata
            .additional_metadata()
            .code_challenge_methods_supported
            .iter()
            .any(|method| method == "S256"),
    };
    if metadata.token_endpoint().is_none() {
        return Err(SsoError::Provider(
            "the provider names no token endpoint".to_owned(),
        ));
    }
    let redirect = RedirectUrl::new(redirect.to_owned())
        .map_err(|error| SsoError::Provider(format!("the public address: {error}")))?;
    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(oidc.client_id.clone()),
        Some(ClientSecret::new(oidc.client_secret.clone())),
    )
    .set_redirect_uri(redirect.clone());
    Ok(Provider {
        oidc: oidc.clone(),
        redirect: redirect.to_string(),
        client,
        report,
    })
}

impl Sso {
    /// # Panics
    ///
    /// When no HTTP client can be made (no TLS roots, say).
    #[must_use]
    pub fn new(settings: Arc<Settings>) -> Arc<Self> {
        let http = reqwest::Client::builder()
            // The flow's redirects are the browser's; following them here
            // would open the door to server-side request forgery.
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            // ASCII: some servers drop requests with other bytes in headers.
            .user_agent(concat!("pixiu/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("an HTTP client");
        Arc::new(Self {
            settings,
            http,
            provider: tokio::sync::Mutex::default(),
            pending: Mutex::default(),
            codes: Mutex::default(),
        })
    }

    /// The provider as set up now, discovered if need be (or `fresh`).
    async fn provider(&self, fresh: bool) -> Result<Arc<Provider>, SsoError> {
        let settings = self.settings.get();
        let (Some(oidc), Some(redirect)) = (settings.sso(), settings.redirect_uri()) else {
            return Err(SsoError::Unavailable);
        };
        let mut cached = self.provider.lock().await;
        if !fresh
            && let Some(provider) = cached.as_ref()
            && provider.oidc == *oidc
            && provider.redirect == redirect
        {
            return Ok(Arc::clone(provider));
        }
        let provider = Arc::new(discover(&self.http, oidc, &redirect).await?);
        *cached = Some(Arc::clone(&provider));
        Ok(provider)
    }

    /// Checks a provider's setup: discovery, keys, PKCE.
    ///
    /// # Errors
    ///
    /// Fails, saying why, when the provider cannot be used.
    pub async fn test(&self, oidc: &Oidc) -> Result<Report, SsoError> {
        let redirect = self
            .settings
            .get()
            .redirect_uri()
            .unwrap_or_else(|| "http://localhost/api/auth/oidc/callback".to_owned());
        Ok(discover(&self.http, oidc, &redirect).await?.report)
    }

    /// Starts a sign-in: where to send the browser, and the value of the
    /// cookie that binds the sign-in to it.
    ///
    /// # Errors
    ///
    /// Fails when single sign-on is not set up or the provider cannot be
    /// reached.
    pub async fn start(&self, intent: Intent) -> Result<Started, SsoError> {
        let provider = self.provider(false).await?;
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let mut request = provider.client.authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        );
        let scopes: Vec<String> = if provider.oidc.scopes.is_empty() {
            DEFAULT_SCOPES.map(str::to_owned).to_vec()
        } else {
            provider.oidc.scopes.clone()
        };
        for scope in scopes.into_iter().filter(|scope| scope != "openid") {
            request = request.add_scope(Scope::new(scope));
        }
        let (url, state, nonce) = request.set_pkce_challenge(challenge).url();
        let binding = random_token();
        let mut pending = self.pending.lock().unwrap();
        pending.retain(|_, waiting| waiting.started.elapsed() < PENDING_FOR);
        if pending.len() >= MAX_PENDING
            && let Some(oldest) = pending
                .iter()
                .min_by_key(|(_, waiting)| waiting.started)
                .map(|(state, _)| state.clone())
        {
            pending.remove(&oldest);
        }
        pending.insert(
            state.secret().clone(),
            Pending {
                intent,
                nonce,
                verifier,
                binding: ApiKey::hash(&binding),
                started: Instant::now(),
            },
        );
        Ok(Started {
            url: url.to_string(),
            binding,
        })
    }

    /// Takes the sign-in `state` names: it is used up either way.
    #[must_use]
    pub fn take(&self, state: &str) -> Option<Pending> {
        self.pending
            .lock()
            .unwrap()
            .remove(state)
            .filter(|pending| pending.started.elapsed() < PENDING_FOR)
    }

    /// Finishes a sign-in: trades the provider's code for its ID token,
    /// and checks it (signature, issuer, audience, expiry, nonce).
    ///
    /// # Errors
    ///
    /// Fails when the browser is not the one that started it, or the
    /// provider or its token fails.
    pub async fn complete(
        &self,
        pending: Pending,
        code: &str,
        binding: Option<&str>,
    ) -> Result<Identity, SsoError> {
        if binding.map(ApiKey::hash).as_deref() != Some(pending.binding.as_str()) {
            return Err(SsoError::Expired);
        }
        let provider = self.provider(false).await?;
        let response = provider
            .client
            .exchange_code(AuthorizationCode::new(code.to_owned()))
            .map_err(|error| SsoError::Provider(explain(&error)))?
            .set_pkce_verifier(pending.verifier)
            .request_async(&self.http)
            .await
            .map_err(|error| {
                SsoError::Provider(format!("the token request failed: {}", explain(&error)))
            })?;
        let token = response
            .id_token()
            .ok_or_else(|| SsoError::Provider("the provider sent no ID token".to_owned()))?;
        let identity = |client: &Client| {
            token
                .claims(&client.id_token_verifier(), &pending.nonce)
                .map(|claims| Identity {
                    issuer: claims.issuer().to_string(),
                    subject: claims.subject().to_string(),
                    email: claims.email().map(|email| email.to_string()),
                })
        };
        match identity(&provider.client) {
            Ok(identity) => Ok(identity),
            // Its keys rotated since discovery: look again, once.
            Err(ClaimsVerificationError::SignatureVerification(
                SignatureVerificationError::NoMatchingKey,
            )) => identity(&self.provider(true).await?.client)
                .map_err(|error| SsoError::Provider(format!("the ID token: {}", explain(&error)))),
            Err(error) => Err(SsoError::Provider(format!(
                "the ID token: {}",
                explain(&error)
            ))),
        }
    }

    /// A code the player trades for `user_id`'s token, once, soon.
    #[must_use]
    pub fn issue_code(&self, user_id: u64) -> String {
        let code = random_token();
        let mut codes = self.codes.lock().unwrap();
        codes.retain(|_, (_, made)| made.elapsed() < CODE_FOR);
        codes.insert(ApiKey::hash(&code), (user_id, Instant::now()));
        code
    }

    /// The user a code is for, if it is still good; it is used up.
    #[must_use]
    pub fn redeem_code(&self, code: &str) -> Option<u64> {
        self.codes
            .lock()
            .unwrap()
            .remove(&ApiKey::hash(code.trim()))
            .filter(|(_, made)| made.elapsed() < CODE_FOR)
            .map(|(user_id, _)| user_id)
    }
}
