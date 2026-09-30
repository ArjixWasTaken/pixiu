//! Emails with links in them, and what following the links does:
//! confirming an address, and choosing a new password.

use pixiu_core::SecretBox;
use pixiu_db::{Db, TokenPurpose, User, UserStatus, now, toasty};

use crate::{
    AccountError, Mailer,
    mail::templates,
    tokens,
    users::{self, check_password},
};

/// Where a user's email goes, if email can reach them.
fn address<'a>(mailer: &Mailer, user: &'a User) -> Result<&'a str, AccountError> {
    if !mailer.ready() {
        return Err(AccountError::Invalid(
            "Email is not set up on this server.".to_owned(),
        ));
    }
    user.email
        .as_deref()
        .ok_or_else(|| AccountError::Invalid("The account has no email address.".to_owned()))
}

/// Emails `user` a link confirming their address.
///
/// # Errors
///
/// Fails when email is not set up, the user has no address, or on database
/// errors.
pub async fn send_verification(
    db: &mut Db,
    mailer: &Mailer,
    user: &User,
) -> Result<(), AccountError> {
    let to = address(mailer, user)?;
    let token = tokens::issue(db, user.id, TokenPurpose::VerifyEmail, Some(to.to_owned())).await?;
    let link = mailer
        .settings()
        .get()
        .link(&format!("/verify-email/{token}"))
        .unwrap_or_default();
    mailer.queue(templates::verify_email(to, &user.username, &link));
    tracing::info!(user = user.id, "confirmation email queued");
    Ok(())
}

/// Emails `user` a link to choose a new password.
///
/// # Errors
///
/// Fails when email is not set up, the user has no address, or on database
/// errors.
pub async fn send_reset(db: &mut Db, mailer: &Mailer, user: &User) -> Result<(), AccountError> {
    let to = address(mailer, user)?;
    let token = tokens::issue(
        db,
        user.id,
        TokenPurpose::ResetPassword,
        Some(to.to_owned()),
    )
    .await?;
    let link = mailer
        .settings()
        .get()
        .link(&format!("/reset-password/{token}"))
        .unwrap_or_default();
    mailer.queue(templates::password_reset(to, &user.username, &link));
    tracing::info!(user = user.id, "password reset email queued");
    Ok(())
}

/// Whether someone who forgot their password gets a reset link: their
/// account works (or only waits for its address to be confirmed), and the
/// address is one they confirmed. An unconfirmed address may be mistyped,
/// and a reset link there would hand the account to a stranger.
#[must_use]
pub fn may_reset(user: &User) -> bool {
    user.email.is_some()
        && match user.status {
            UserStatus::Active => user.email_verified_at.is_some(),
            UserStatus::Unverified => true,
            UserStatus::Pending | UserStatus::Disabled => false,
        }
}

/// Following a link sent to `email` proves it reaches `user`: confirms the
/// address, if it is still theirs, and activates an account that only
/// waited for that.
async fn confirm(db: &mut Db, mut user: User, email: Option<&str>) -> Result<User, AccountError> {
    if email.is_none() || user.email.as_deref() != email {
        return Ok(user);
    }
    let email_verified_at = user.email_verified_at.or_else(|| Some(now()));
    let status = match user.status {
        UserStatus::Unverified => UserStatus::Active,
        status => status,
    };
    toasty::update!(user {
        email_verified_at,
        status,
    })
    .exec(db)
    .await?;
    Ok(user)
}

async fn owner(
    db: &mut Db,
    redeemed: Option<tokens::Redeemed>,
) -> Result<(User, Option<String>), AccountError> {
    let redeemed = redeemed.ok_or(AccountError::LinkExpired)?;
    let user = User::filter_by_id(redeemed.user_id)
        .first()
        .exec(db)
        .await?
        .ok_or(AccountError::LinkExpired)?;
    Ok((user, redeemed.email))
}

/// Follows a confirmation link.
///
/// # Errors
///
/// Fails when the link expired, was used, or is for an address the account
/// no longer has; or on database errors.
pub async fn verify_email(db: &mut Db, token: &str) -> Result<User, AccountError> {
    let redeemed = tokens::redeem(db, token, TokenPurpose::VerifyEmail).await?;
    let (user, email) = owner(db, redeemed).await?;
    if email.is_none() || user.email != email {
        return Err(AccountError::LinkExpired);
    }
    let user = confirm(db, user, email.as_deref()).await?;
    tracing::info!(user = user.id, "email confirmed");
    Ok(user)
}

/// Follows a reset link: `password` becomes the account's, and every web
/// session signs out.
///
/// # Errors
///
/// Fails when the password breaks the rules (the link still works then),
/// the link expired or was used, or on database errors.
pub async fn reset_password(
    db: &mut Db,
    secrets: &SecretBox,
    token: &str,
    password: &str,
) -> Result<User, AccountError> {
    check_password(password)?;
    let redeemed = tokens::redeem(db, token, TokenPurpose::ResetPassword).await?;
    let (user, email) = owner(db, redeemed).await?;
    let user = users::set_password(db, secrets, user, password, false, None).await?;
    confirm(db, user, email.as_deref()).await
}
