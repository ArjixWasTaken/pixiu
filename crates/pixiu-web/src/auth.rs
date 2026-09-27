//! Authentication, modeled as request functions (Topcoat's "functions, not
//! middlewares"): every handler that needs the admin calls [`require_user`].

use std::sync::LazyLock;

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier},
};
use pixiu_db::{Db, User, WebSession, now, toasty};
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

/// Hashes a password with argon2id. Runs off the async runtime: hashing is
/// deliberately slow.
pub(crate) async fn hash_password(password: String) -> Result<String> {
    let hash = tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|hash| hash.to_string())
    })
    .await??;
    Ok(hash)
}

/// Checks `password` against a stored PHC hash. Pass `None` for unknown
/// users so a miss costs as much time as a wrong password.
pub(crate) async fn verify_password(password: String, hash: Option<String>) -> Result<bool> {
    // Verified against when the user is unknown, so both paths do the same
    // argon2 work and response times do not reveal valid usernames.
    static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| {
        Argon2::default()
            .hash_password(b"not the password")
            .expect("hashing with default parameters succeeds")
            .to_string()
    });

    let known = hash.is_some();
    let matches = tokio::task::spawn_blocking(move || {
        let hash = hash.as_deref().unwrap_or(&DUMMY_HASH);
        Argon2::default()
            .verify_password(password.as_bytes(), hash)
            .is_ok()
    })
    .await?;
    Ok(known && matches)
}
