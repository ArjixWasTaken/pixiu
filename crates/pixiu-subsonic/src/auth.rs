//! Request authentication: OpenSubsonic API keys, token authentication
//! (`t` = md5(password + `s`)) and plain passwords (`p`, optionally
//! hex-encoded as `enc:...`).

use md5::{Digest, Md5};
use pixiu_db::{ApiKey, User, now, toasty};

use crate::{
    Failure, Params, SubsonicState,
    response::{ApiError, ErrorCode},
};

/// Resolves the user a request authenticates as.
pub(crate) async fn authenticate(state: &SubsonicState, params: &Params) -> Result<User, Failure> {
    let mut db = state.db.clone();
    match (params.get("apiKey"), params.get("u")) {
        (Some(_), Some(_)) => Err(ApiError::new(
            ErrorCode::ConflictingAuth,
            "use either an API key or a username, not both",
        )
        .into()),
        (Some(key), None) => {
            let Some(api_key) = ApiKey::filter_by_key_hash(ApiKey::hash(key))
                .first()
                .exec(&mut db)
                .await?
            else {
                return Err(ApiError::new(ErrorCode::InvalidApiKey, "invalid API key").into());
            };
            let user = User::get_by_id(&mut db, &api_key.user_id).await?;
            touch(&mut db, api_key).await;
            Ok(user)
        }
        (None, Some(username)) => {
            let user = User::filter_by_username(username)
                .first()
                .exec(&mut db)
                .await?;
            match (params.get("t"), params.get("s"), params.get("p")) {
                (Some(token), Some(salt), _) => check_token(state, user, token, salt),
                (_, _, Some(password)) => check_password(state, user, password).await,
                _ => Err(ApiError::missing_parameter("p` or `t` and `s").into()),
            }
        }
        (None, None) => Err(ApiError::missing_parameter("u` or `apiKey").into()),
    }
}

fn wrong_credentials() -> Failure {
    ApiError::new(ErrorCode::WrongCredentials, "wrong username or password").into()
}

fn check_token(
    state: &SubsonicState,
    user: Option<User>,
    token: &str,
    salt: &str,
) -> Result<User, Failure> {
    let Some(user) = user else {
        return Err(wrong_credentials());
    };
    let Some(sealed) = &user.subsonic_secret else {
        return Err(ApiError::new(
            ErrorCode::TokenAuthNotSupported,
            "token authentication needs the password; sign in to the WebUI once, \
             or use an API key",
        )
        .into());
    };
    let password = state.secrets.open_str(sealed).map_err(|error| {
        tracing::error!(%error, "cannot open the sealed Subsonic password");
        Failure::from(ApiError::new(ErrorCode::Generic, "internal error"))
    })?;
    let expected = hex::encode(Md5::digest(format!("{password}{salt}")));
    if constant_time_eq(expected.as_bytes(), token.to_ascii_lowercase().as_bytes()) {
        Ok(user)
    } else {
        Err(wrong_credentials())
    }
}

async fn check_password(
    state: &SubsonicState,
    user: Option<User>,
    password: &str,
) -> Result<User, Failure> {
    let password = match password.strip_prefix("enc:") {
        Some(encoded) => hex::decode(encoded)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .ok_or_else(wrong_credentials)?,
        None => password.to_owned(),
    };

    let hash = user.as_ref().map(|user| user.password_hash.clone());
    let candidate = password.clone();
    let verified = tokio::task::spawn_blocking(move || {
        pixiu_core::password::verify(&candidate, hash.as_deref())
    })
    .await
    .map_err(|_| ApiError::new(ErrorCode::Generic, "internal error"))?;

    match user {
        Some(mut user) if verified => {
            // Capture the password so token authentication works later.
            if user.subsonic_secret.is_none() {
                let sealed = state.secrets.seal_str(&password);
                toasty::update!(user {
                    subsonic_secret: Some(sealed)
                })
                .exec(&mut state.db.clone())
                .await?;
            }
            Ok(user)
        }
        _ => Err(wrong_credentials()),
    }
}

/// Records when a key was last used, at most once a minute.
async fn touch(db: &mut pixiu_db::Db, mut api_key: ApiKey) {
    let stale = api_key
        .last_used_at
        .is_none_or(|last| now().duration_since(last).as_secs() >= 60);
    if stale
        && let Err(error) = toasty::update!(api_key {
            last_used_at: Some(now())
        })
        .exec(db)
        .await
    {
        tracing::warn!(%error, "failed to record API key use");
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_in_constant_time() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }

    #[test]
    fn token_matches_the_subsonic_example() {
        // From the Subsonic API docs: password "sesame", salt "c19b2d".
        let token = hex::encode(Md5::digest("sesamec19b2d"));
        assert_eq!(token, "26719a1196d2a940705a59634eb18eab");
    }
}
