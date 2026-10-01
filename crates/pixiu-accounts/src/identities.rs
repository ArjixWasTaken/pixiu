//! Accounts at the single sign-on provider, linked to píxiū accounts. Each
//! provider account links to one píxiū account, and each píxiū account to
//! one account per provider.

use pixiu_db::{Db, User, UserIdentity, UserStatus, now, toasty};

use crate::{AccountError, oidc::Identity};

/// The píxiū account `identity` is linked to, and the link.
///
/// # Errors
///
/// Fails on database errors.
pub async fn owner(
    db: &mut Db,
    identity: &Identity,
) -> Result<Option<(UserIdentity, User)>, toasty::Error> {
    let Some(link) =
        UserIdentity::filter_by_issuer_and_subject(&identity.issuer, &identity.subject)
            .first()
            .exec(db)
            .await?
    else {
        return Ok(None);
    };
    let user = User::filter_by_id(link.user_id).first().exec(db).await?;
    Ok(user.map(|user| (link, user)))
}

/// Notes a sign-in through `link`.
///
/// # Errors
///
/// Fails on database errors.
pub async fn signed_in(
    db: &mut Db,
    mut link: UserIdentity,
    identity: &Identity,
) -> Result<(), toasty::Error> {
    toasty::update!(link {
        last_login_at: Some(now()),
        email: identity.email.clone(),
    })
    .exec(db)
    .await
}

/// Links `identity` to `user_id`'s account, in place of any other account
/// of theirs at that provider.
///
/// # Errors
///
/// Fails with [`AccountError::IdentityTaken`] when it is linked to someone
/// else, [`AccountError::NotFound`] when the account is gone or off, or on
/// database errors.
pub async fn link(
    db: &mut Db,
    user_id: u64,
    identity: &Identity,
) -> Result<UserIdentity, AccountError> {
    let mut tx = pixiu_db::write_transaction(db).await?;
    let active = User::filter_by_id(user_id)
        .first()
        .exec(&mut tx)
        .await?
        .is_some_and(|user| user.status == UserStatus::Active);
    if !active {
        return Err(AccountError::NotFound);
    }
    if let Some(existing) =
        UserIdentity::filter_by_issuer_and_subject(&identity.issuer, &identity.subject)
            .first()
            .exec(&mut tx)
            .await?
    {
        if existing.user_id != user_id {
            return Err(AccountError::IdentityTaken);
        }
        existing.delete().exec(&mut tx).await?;
    }
    if let Some(other) = UserIdentity::filter_by_user_id_and_issuer(user_id, &identity.issuer)
        .first()
        .exec(&mut tx)
        .await?
    {
        other.delete().exec(&mut tx).await?;
    }
    let link = toasty::create!(UserIdentity {
        user_id,
        issuer: identity.issuer.clone(),
        subject: identity.subject.clone(),
        email: identity.email.clone(),
        linked_at: now(),
    })
    .exec(&mut tx)
    .await?;
    tx.commit().await?;
    tracing::info!(user = user_id, issuer = %identity.issuer, "single sign-on linked");
    Ok(link)
}

/// `user_id`'s linked accounts, oldest first.
///
/// # Errors
///
/// Fails on database errors.
pub async fn of(db: &mut Db, user_id: u64) -> Result<Vec<UserIdentity>, toasty::Error> {
    let mut links = UserIdentity::filter_by_user_id(user_id).exec(db).await?;
    links.sort_by_key(|link| link.linked_at);
    Ok(links)
}

/// Unlinks one of `user_id`'s linked accounts.
///
/// # Errors
///
/// Fails with [`AccountError::NotFound`] when it is not theirs, or on
/// database errors.
pub async fn unlink(db: &mut Db, user_id: u64, id: u64) -> Result<(), AccountError> {
    let link = UserIdentity::filter_by_id(id)
        .first()
        .exec(db)
        .await?
        .filter(|link| link.user_id == user_id)
        .ok_or(AccountError::NotFound)?;
    link.delete().exec(db).await?;
    tracing::info!(user = user_id, "single sign-on unlinked");
    Ok(())
}
