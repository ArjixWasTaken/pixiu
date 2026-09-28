//! Authentication, modeled as request functions (Topcoat's "functions, not
//! middlewares"): every handler that needs the admin calls [`require_user`].

use std::sync::Arc;

use pixiu_browser::LoginDesk;
use pixiu_core::{SecretBox, playing::NowPlaying};
use pixiu_db::{Db, User, WebSession, now, toasty};
use pixiu_hunt::Hunter;
use pixiu_jobs::{Jobs, Warden};
use pixiu_treasury::{Offerings, Treasury};
use topcoat::{
    Result,
    context::{Cx, app_context, memoize},
    router::error::redirect,
    session,
};

/// Paths the auth flow redirects to.
pub(crate) const LOGIN_PATH: &str = "/login";
pub(crate) const SETUP_PATH: &str = "/setup";

/// The database handle. Cloning is cheap; it shares the pool.
pub(crate) fn db(cx: &Cx) -> Db {
    app_context::<Db>(cx).clone()
}

pub(crate) fn secrets(cx: &Cx) -> &SecretBox {
    app_context(cx)
}

pub(crate) fn treasury(cx: &Cx) -> &Treasury {
    app_context(cx)
}

pub(crate) fn offerings(cx: &Cx) -> &Offerings {
    app_context(cx)
}

pub(crate) fn hunter(cx: &Cx) -> &Hunter {
    app_context::<Arc<Hunter>>(cx)
}

pub(crate) fn warden(cx: &Cx) -> &Arc<Warden> {
    app_context(cx)
}

pub(crate) fn jobs(cx: &Cx) -> &Arc<Jobs> {
    app_context(cx)
}

pub(crate) fn login_desk(cx: &Cx) -> &Arc<LoginDesk> {
    app_context(cx)
}

pub(crate) fn now_playing(cx: &Cx) -> &NowPlaying {
    app_context(cx)
}

/// The logged-in admin, if the request carries a live session.
#[memoize(as_ref)]
pub(crate) async fn current_user(cx: &Cx) -> Result<Option<User>> {
    let Some(hash) = session::token_hash(cx).await? else {
        return Ok(None);
    };
    let mut db = db(cx);
    let Some(record) = WebSession::filter_by_token_hash(hex::encode(*hash))
        .first()
        .exec(&mut db)
        .await?
    else {
        return Ok(None);
    };
    if record.expires_at <= now() {
        WebSession::delete_by_token_hash(&mut db, &record.token_hash).await?;
        return Ok(None);
    }
    Ok(Some(User::get_by_id(&mut db, &record.user_id).await?))
}

/// The logged-in admin, or a redirect to the login page.
pub(crate) async fn require_user(cx: &Cx) -> Result<&User> {
    match current_user(cx).await {
        Ok(Some(user)) => Ok(user),
        Ok(None) => Err(redirect(LOGIN_PATH).into()),
        Err(error) => Err(error.clone()),
    }
}

/// Whether first-run setup has created the admin account.
pub(crate) async fn is_claimed(cx: &Cx) -> Result<bool> {
    Ok(User::all().first().exec(&mut db(cx)).await?.is_some())
}

/// Issues a fresh session cookie for `user` and records its hash.
pub(crate) async fn start_session(cx: &Cx, user: &User) -> Result<()> {
    let session = session::start(cx).await?;
    let expires_at = jiff::Timestamp::try_from(session.expires_at)?;
    toasty::create!(WebSession {
        token_hash: hex::encode(*session.token_hash),
        user_id: user.id,
        created_at: now(),
        expires_at,
    })
    .exec(&mut db(cx))
    .await?;
    Ok(())
}

/// Clears the session cookie and forgets the session.
pub(crate) async fn end_session(cx: &Cx) -> Result<()> {
    if let Some(hash) = session::stop(cx).await? {
        WebSession::delete_by_token_hash(&mut db(cx), hex::encode(*hash)).await?;
    }
    Ok(())
}

/// Hashes a password with argon2id, off the async runtime: hashing is
/// deliberately slow.
pub(crate) async fn hash_password(password: String) -> Result<String> {
    Ok(tokio::task::spawn_blocking(move || pixiu_core::password::hash(&password)).await?)
}

/// Checks `password` against a stored hash; see
/// [`pixiu_core::password::verify`].
pub(crate) async fn verify_password(password: String, hash: Option<String>) -> Result<bool> {
    Ok(tokio::task::spawn_blocking(move || {
        pixiu_core::password::verify(&password, hash.as_deref())
    })
    .await?)
}

/// Keeps the password, sealed, for Subsonic token authentication.
pub(crate) async fn remember_password(cx: &Cx, user: &mut User, password: &str) -> Result<()> {
    let sealed = secrets(cx).seal_str(password);
    toasty::update!(user {
        subsonic_secret: Some(sealed)
    })
    .exec(&mut db(cx))
    .await?;
    Ok(())
}
