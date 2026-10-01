//! The server's settings, for admins: the address people reach píxiū at,
//! registration, the mail server and the single sign-on provider. Secrets
//! are never sent back; the player learns only whether one is set.

use axum::{Json, extract::State, http::StatusCode};
use pixiu_accounts::{
    Oidc, Security, ServerSettings, Smtp, mail::templates, oidc::SsoError, users::check_email,
};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::{AdminSession, ApiError, ApiResult, ApiState};

fn security_name(security: Security) -> &'static str {
    match security {
        Security::None => "none",
        Security::StartTls => "starttls",
        Security::Tls => "tls",
    }
}

fn describe(settings: &ServerSettings) -> JsonValue {
    json!({
        "public_url": settings.public_url,
        "registration_open": settings.registration_open,
        "mail_ready": settings.mail_ready(),
        "smtp": settings.smtp.as_ref().map(|smtp| json!({
            "host": smtp.host,
            "port": smtp.port,
            "security": security_name(smtp.security),
            "username": smtp.username,
            "password_set": smtp.password.is_some(),
            "from": smtp.from,
        })),
        "oidc": settings.oidc.as_ref().map(|oidc| json!({
            "name": oidc.name,
            "issuer": oidc.issuer,
            "client_id": oidc.client_id,
            "secret_set": true,
            "scopes": oidc.scopes,
        })),
        // What to give the provider as the client's redirect URI.
        "redirect_uri": settings.redirect_uri(),
    })
}

/// `GET /api/admin/settings`.
pub(crate) async fn show(State(state): State<ApiState>, _: AdminSession) -> Json<JsonValue> {
    Json(describe(&state.settings.get()))
}

#[derive(Deserialize)]
pub(crate) struct Server {
    public_url: Option<String>,
}

/// `PUT /api/admin/settings/server`: where people reach píxiū.
pub(crate) async fn set_server(
    State(state): State<ApiState>,
    _: AdminSession,
    Json(server): Json<Server>,
) -> ApiResult<Json<JsonValue>> {
    state
        .settings
        .set_public_url(server.public_url.as_deref())
        .await?;
    Ok(Json(describe(&state.settings.get())))
}

#[derive(Deserialize)]
pub(crate) struct SmtpForm {
    host: String,
    port: u16,
    security: String,
    #[serde(default)]
    username: Option<String>,
    /// Left out (or `null`) keeps the stored one; empty removes it.
    #[serde(default)]
    password: Option<String>,
    from: String,
}

/// `PUT /api/admin/settings/smtp`: the mail server.
pub(crate) async fn set_smtp(
    State(state): State<ApiState>,
    _: AdminSession,
    Json(form): Json<SmtpForm>,
) -> ApiResult<Json<JsonValue>> {
    let security = match form.security.as_str() {
        "none" => Security::None,
        "starttls" => Security::StartTls,
        "tls" => Security::Tls,
        other => {
            return Err(ApiError::unprocessable(format!(
                "`{other}` is not a way to secure mail"
            )));
        }
    };
    let password = match form.password {
        None => state
            .settings
            .get()
            .smtp
            .as_ref()
            .and_then(|smtp| smtp.password.clone()),
        Some(password) if password.is_empty() => None,
        Some(password) => Some(password),
    };
    let username = form
        .username
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty());
    state
        .settings
        .set_smtp(Some(Smtp {
            host: form.host.trim().to_owned(),
            port: form.port,
            security,
            username,
            password,
            from: form.from.trim().to_owned(),
        }))
        .await?;
    Ok(Json(describe(&state.settings.get())))
}

/// `DELETE /api/admin/settings/smtp`: no more email.
pub(crate) async fn remove_smtp(
    State(state): State<ApiState>,
    _: AdminSession,
) -> ApiResult<Json<JsonValue>> {
    state.settings.set_smtp(None).await?;
    Ok(Json(describe(&state.settings.get())))
}

#[derive(Deserialize, Default)]
pub(crate) struct Test {
    /// Where to; the admin's own address when left out.
    #[serde(default)]
    to: Option<String>,
}

/// `POST /api/admin/settings/smtp/test`: sends an email now, answering the
/// mail server's error when it fails.
pub(crate) async fn test_smtp(
    State(state): State<ApiState>,
    AdminSession(admin): AdminSession,
    Json(test): Json<Test>,
) -> ApiResult<StatusCode> {
    let to = match test.to.as_deref().map(check_email).transpose()?.flatten() {
        Some(to) => to,
        None => admin.user.email.clone().ok_or_else(|| {
            ApiError::unprocessable(
                "Say where to send it, or add your email address under Account.",
            )
        })?,
    };
    state
        .mailer
        .send_now(&templates::test(&to))
        .await
        .map_err(|error| {
            tracing::warn!(%error, "test email failed");
            ApiError::unprocessable(error).with_code("smtp")
        })?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub(crate) struct Registration {
    open: bool,
}

/// `PUT /api/admin/settings/registration`: lets anyone ask for an account,
/// or not. Opening it needs email, which tells applicants how it went.
pub(crate) async fn set_registration(
    State(state): State<ApiState>,
    _: AdminSession,
    Json(registration): Json<Registration>,
) -> ApiResult<Json<JsonValue>> {
    if registration.open && !state.settings.get().mail_ready() {
        return Err(ApiError::unprocessable(
            "Registration needs email: set up a mail server and the public address first.",
        ));
    }
    state
        .settings
        .set_registration_open(registration.open)
        .await?;
    Ok(Json(describe(&state.settings.get())))
}

#[derive(Deserialize)]
pub(crate) struct OidcForm {
    name: String,
    issuer: String,
    client_id: String,
    /// Left out (or empty) keeps the stored one.
    #[serde(default)]
    client_secret: Option<String>,
    #[serde(default)]
    scopes: Vec<String>,
}

impl OidcForm {
    /// The provider the form describes, with the stored secret unless it
    /// gives another.
    fn provider(self, state: &ApiState) -> ApiResult<Oidc> {
        // Secrets are pasted, often with a space or a line break around them.
        let client_secret = match self
            .client_secret
            .map(|secret| secret.trim().to_owned())
            .filter(|secret| !secret.is_empty())
        {
            Some(secret) => secret,
            None => state
                .settings
                .get()
                .oidc
                .as_ref()
                .map(|oidc| oidc.client_secret.clone())
                .ok_or_else(|| ApiError::unprocessable("Give the client secret."))?,
        };
        Ok(Oidc {
            name: self.name,
            issuer: self.issuer,
            client_id: self.client_id,
            client_secret,
            scopes: self
                .scopes
                .into_iter()
                .flat_map(|scope| {
                    scope
                        .split_whitespace()
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .collect(),
        })
    }
}

/// `PUT /api/admin/settings/oidc`: the single sign-on provider.
pub(crate) async fn set_oidc(
    State(state): State<ApiState>,
    _: AdminSession,
    Json(form): Json<OidcForm>,
) -> ApiResult<Json<JsonValue>> {
    let oidc = form.provider(&state)?;
    state.settings.set_oidc(Some(oidc)).await?;
    Ok(Json(describe(&state.settings.get())))
}

/// `DELETE /api/admin/settings/oidc`: no more single sign-on. Links stay,
/// for a provider set up again.
pub(crate) async fn remove_oidc(
    State(state): State<ApiState>,
    _: AdminSession,
) -> ApiResult<Json<JsonValue>> {
    state.settings.set_oidc(None).await?;
    Ok(Json(describe(&state.settings.get())))
}

/// `POST /api/admin/settings/oidc/test`: checks a provider (as in the
/// form, saved or not): its discovery document, keys, and PKCE.
pub(crate) async fn test_oidc(
    State(state): State<ApiState>,
    _: AdminSession,
    Json(form): Json<OidcForm>,
) -> ApiResult<Json<pixiu_accounts::oidc::Report>> {
    let oidc = form.provider(&state)?;
    state
        .sso
        .test(&oidc)
        .await
        .map(Json)
        .map_err(|error| match error {
            SsoError::Provider(message) => ApiError::unprocessable(message).with_code("provider"),
            error => ApiError::unprocessable(error.to_string()),
        })
}
