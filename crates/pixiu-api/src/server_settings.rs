//! The server's settings, for admins: the address people reach píxiū at and
//! the mail server. Secrets are never sent back; the player learns only
//! whether one is set.

use axum::{Json, extract::State, http::StatusCode};
use pixiu_accounts::{Security, ServerSettings, Smtp, mail::templates, users::check_email};
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
