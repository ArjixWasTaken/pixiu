//! Single-use links sent by email: confirming an address (7 days) and
//! resetting a password (an hour). Only a token's hash is stored; a new one
//! replaces the user's earlier ones of its kind.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jiff::{SignedDuration, Timestamp};
use pixiu_db::{AccountToken, ApiKey, Db, TokenPurpose, now, toasty};

/// How long a link works.
#[must_use]
pub fn lifetime(purpose: TokenPurpose) -> SignedDuration {
    match purpose {
        TokenPurpose::VerifyEmail => SignedDuration::from_hours(7 * 24),
        TokenPurpose::ResetPassword => SignedDuration::from_hours(1),
    }
}

/// Makes a token for `user_id`, returning it in the clear (for the link),
/// once. `email` is the address a confirmation confirms.
///
/// # Errors
///
/// Fails on database errors.
pub async fn issue(
    db: &mut Db,
    user_id: u64,
    purpose: TokenPurpose,
    email: Option<String>,
) -> Result<String, toasty::Error> {
    let token = URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>());
    let mut tx = pixiu_db::write_transaction(db).await?;
    for old in AccountToken::filter_by_user_id(user_id)
        .exec(&mut tx)
        .await?
    {
        if old.purpose == purpose {
            old.delete().exec(&mut tx).await?;
        }
    }
    let created = now();
    toasty::create!(AccountToken {
        user_id,
        purpose,
        token_hash: ApiKey::hash(&token),
        email,
        created_at: created,
        expires_at: created
            .checked_add(lifetime(purpose))
            .unwrap_or(Timestamp::MAX),
    })
    .exec(&mut tx)
    .await?;
    tx.commit().await?;
    Ok(token)
}

/// What a used-up token was for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redeemed {
    pub user_id: u64,
    /// For email confirmations, the address confirmed.
    pub email: Option<String>,
}

/// Uses up a token: the stored one, if it is for `purpose` and has not
/// expired. It works once.
///
/// # Errors
///
/// Fails on database errors.
pub async fn redeem(
    db: &mut Db,
    token: &str,
    purpose: TokenPurpose,
) -> Result<Option<Redeemed>, toasty::Error> {
    // In one transaction, so that two requests with the same link cannot
    // both use it.
    let mut tx = pixiu_db::write_transaction(db).await?;
    let Some(stored) = AccountToken::filter_by_token_hash(ApiKey::hash(token.trim()))
        .first()
        .exec(&mut tx)
        .await?
    else {
        return Ok(None);
    };
    if stored.purpose != purpose {
        return Ok(None);
    }
    let usable = stored.expires_at > now();
    let redeemed = Redeemed {
        user_id: stored.user_id,
        email: stored.email.clone(),
    };
    stored.delete().exec(&mut tx).await?;
    tx.commit().await?;
    Ok(usable.then_some(redeemed))
}
