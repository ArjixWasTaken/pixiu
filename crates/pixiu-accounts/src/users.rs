//! Users: making them, their passwords, roles and statuses, what their
//! libraries take up on disk, and removing them.
//!
//! píxiū always keeps an active admin: the last one cannot be demoted,
//! disabled or deleted.

use std::collections::HashMap;

use pixiu_core::{SecretBox, password};
use pixiu_db::{ApiKey, Db, Role, User, UserStatus, now, owned::as_u64, toasty};

/// Passwords are at least this long.
pub const MIN_PASSWORD_LEN: usize = 8;

/// The API keys the web player signs in with, which a new password revokes.
pub const WEB_SESSION: &str = "Web session";

#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    /// A value breaks a rule; the message says which, for people.
    #[error("{0}")]
    Invalid(String),
    #[error("That username is taken.")]
    UsernameTaken,
    #[error("That email address is taken.")]
    EmailTaken,
    #[error("píxiū needs an active admin; make someone else an admin first.")]
    LastAdmin,
    #[error("no such user")]
    NotFound,
    #[error("píxiū is set up already.")]
    AlreadySetUp,
    #[error("This link has expired or was already used.")]
    LinkExpired,
    #[error("Registration is closed.")]
    RegistrationClosed,
    #[error("That request was handled already.")]
    NotPending,
    #[error("database error: {0}")]
    Db(#[from] toasty::Error),
    #[error("background task failed: {0}")]
    Join(#[from] tokio::task::JoinError),
}

/// A username, trimmed, if it follows the rules: 1 to 64 characters, no
/// `@` (signing in takes a username or an email) and nothing invisible.
///
/// # Errors
///
/// Fails, with a message for people, when it does not.
pub fn check_username(name: &str) -> Result<String, AccountError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AccountError::Invalid("Choose a username.".to_owned()));
    }
    if name.chars().count() > 64 {
        return Err(AccountError::Invalid(
            "A username is at most 64 characters.".to_owned(),
        ));
    }
    if name.contains('@') || name.chars().any(char::is_control) {
        return Err(AccountError::Invalid(
            "A username cannot contain @ or invisible characters.".to_owned(),
        ));
    }
    Ok(name.to_owned())
}

/// Checks a new password against the rules.
///
/// # Errors
///
/// Fails, with a message for people, when it is too short.
pub fn check_password(password: &str) -> Result<(), AccountError> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(AccountError::Invalid(format!(
            "A password is at least {MIN_PASSWORD_LEN} characters."
        )));
    }
    Ok(())
}

/// An email address as stored: trimmed and lowercase. Empty means none.
///
/// # Errors
///
/// Fails, with a message for people, when it does not look like an
/// address.
pub fn check_email(email: &str) -> Result<Option<String>, AccountError> {
    let email = email.trim().to_lowercase();
    if email.is_empty() {
        return Ok(None);
    }
    let looks_right = email.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty()
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
    }) && !email.chars().any(|c| c.is_whitespace() || c.is_control())
        && email.len() <= 254;
    if looks_right {
        Ok(Some(email))
    } else {
        Err(AccountError::Invalid(
            "That does not look like an email address.".to_owned(),
        ))
    }
}

/// The user signing in as `login`: a username (any case) or an email.
///
/// # Errors
///
/// Fails on database errors.
pub async fn find_by_login(db: &mut Db, login: &str) -> Result<Option<User>, toasty::Error> {
    let login = login.trim();
    if login.contains('@')
        && let Some(user) = User::filter_by_email(Some(login.to_lowercase()))
            .first()
            .exec(db)
            .await?
    {
        return Ok(Some(user));
    }
    // Usernames made before they could not hold an `@` still sign in.
    if let Some(user) = User::filter_by_username(login).first().exec(db).await? {
        return Ok(Some(user));
    }
    let wanted = login.to_lowercase();
    Ok(User::all()
        .exec(db)
        .await?
        .into_iter()
        .find(|user| user.username.to_lowercase() == wanted))
}

async fn username_taken(
    db: &mut dyn toasty::Executor,
    name: &str,
    except: Option<u64>,
) -> Result<bool, toasty::Error> {
    let wanted = name.to_lowercase();
    Ok(User::all()
        .exec(db)
        .await?
        .iter()
        .any(|user| user.username.to_lowercase() == wanted && Some(user.id) != except))
}

async fn email_taken(
    db: &mut dyn toasty::Executor,
    email: &str,
    except: Option<u64>,
) -> Result<bool, toasty::Error> {
    Ok(User::filter_by_email(Some(email.to_owned()))
        .first()
        .exec(db)
        .await?
        .is_some_and(|user| Some(user.id) != except))
}

/// An account to make.
#[derive(Debug, Clone)]
pub struct NewUser {
    pub username: String,
    pub email: Option<String>,
    /// Whether the email counts as confirmed (an admin typed it).
    pub email_verified: bool,
    pub password: String,
    pub role: Role,
    pub status: UserStatus,
    /// A temporary password: the user picks their own at first sign-in.
    pub password_change_required: bool,
}

/// The stored forms of a password: its hash, and the password sealed for
/// Subsonic token authentication. Slow; runs on a blocking thread.
async fn seal(secrets: &SecretBox, password: &str) -> Result<(String, String), AccountError> {
    let (secrets, password) = (secrets.clone(), password.to_owned());
    Ok(tokio::task::spawn_blocking(move || {
        (password::hash(&password), secrets.seal_str(&password))
    })
    .await?)
}

/// Makes an account.
///
/// # Errors
///
/// Fails when a value breaks a rule, the username or email is taken, or on
/// database errors.
pub async fn create(db: &mut Db, secrets: &SecretBox, new: NewUser) -> Result<User, AccountError> {
    insert(db, secrets, new, false).await
}

/// Makes the first account, an active admin, unless one exists: the check
/// and the insert share a transaction, so two racing setups cannot both
/// make one.
///
/// # Errors
///
/// Fails with [`AccountError::AlreadySetUp`] when an account exists, when a
/// value breaks a rule, or on database errors.
pub async fn create_first(
    db: &mut Db,
    secrets: &SecretBox,
    username: &str,
    password: &str,
) -> Result<User, AccountError> {
    let new = NewUser {
        username: username.to_owned(),
        email: None,
        email_verified: false,
        password: password.to_owned(),
        role: Role::Admin,
        status: UserStatus::Active,
        password_change_required: false,
    };
    insert(db, secrets, new, true).await
}

async fn insert(
    db: &mut Db,
    secrets: &SecretBox,
    new: NewUser,
    first: bool,
) -> Result<User, AccountError> {
    let username = check_username(&new.username)?;
    check_password(&new.password)?;
    let email = match new.email {
        Some(email) => check_email(&email)?,
        None => None,
    };
    let (hash, sealed) = seal(secrets, &new.password).await?;
    let mut tx = pixiu_db::write_transaction(db).await?;
    if first && User::all().first().exec(&mut tx).await?.is_some() {
        return Err(AccountError::AlreadySetUp);
    }
    if username_taken(&mut tx, &username, None).await? {
        return Err(AccountError::UsernameTaken);
    }
    if let Some(email) = &email
        && email_taken(&mut tx, email, None).await?
    {
        return Err(AccountError::EmailTaken);
    }
    let verified_at = (new.email_verified && email.is_some()).then(now);
    let user = toasty::create!(User {
        username,
        email,
        email_verified_at: verified_at,
        role: new.role,
        status: new.status,
        password_change_required: new.password_change_required,
        password_hash: hash,
        subsonic_secret: Some(sealed),
        created_at: now(),
    })
    .exec(&mut tx)
    .await?;
    tx.commit().await?;
    tracing::info!(user = user.id, username = %user.username, "account created");
    Ok(user)
}

/// Gives a user a new password, signing their web sessions out (all but
/// `keep`, the one making the change). `temporary` makes them pick their
/// own at their next sign-in.
///
/// # Errors
///
/// Fails when the password breaks the rules, or on database errors.
pub async fn set_password(
    db: &mut Db,
    secrets: &SecretBox,
    mut user: User,
    password: &str,
    temporary: bool,
    keep: Option<u64>,
) -> Result<User, AccountError> {
    check_password(password)?;
    let (hash, sealed) = seal(secrets, password).await?;
    let mut tx = pixiu_db::write_transaction(db).await?;
    toasty::update!(user {
        password_hash: hash,
        subsonic_secret: Some(sealed),
        password_change_required: temporary,
    })
    .exec(&mut tx)
    .await?;
    for key in ApiKey::filter_by_user_id(user.id).exec(&mut tx).await? {
        if key.name == WEB_SESSION && Some(key.id) != keep {
            key.delete().exec(&mut tx).await?;
        }
    }
    tx.commit().await?;
    tracing::info!(user = user.id, temporary, "password changed");
    Ok(user)
}

/// Changes a user's username and email. A new email is unconfirmed unless
/// `verified`. Only a new username must follow today's rules: names from
/// before them (`me@example.com`) stay.
///
/// # Errors
///
/// Fails when a value breaks a rule or is taken, or on database errors.
pub async fn set_profile(
    db: &mut Db,
    mut user: User,
    username: &str,
    email: &str,
    verified: bool,
) -> Result<User, AccountError> {
    let username = if username.trim() == user.username {
        user.username.clone()
    } else {
        check_username(username)?
    };
    let email = check_email(email)?;
    let mut tx = pixiu_db::write_transaction(db).await?;
    if username_taken(&mut tx, &username, Some(user.id)).await? {
        return Err(AccountError::UsernameTaken);
    }
    if let Some(email) = &email
        && email_taken(&mut tx, email, Some(user.id)).await?
    {
        return Err(AccountError::EmailTaken);
    }
    let changed_email = email != user.email;
    let email_verified_at = match (&email, changed_email) {
        (None, _) => None,
        (Some(_), false) => user.email_verified_at,
        (Some(_), true) => verified.then(now),
    };
    toasty::update!(user {
        username,
        email,
        email_verified_at,
    })
    .exec(&mut tx)
    .await?;
    tx.commit().await?;
    Ok(user)
}

/// Active admins, but `except`.
async fn other_active_admins(
    db: &mut dyn toasty::Executor,
    except: u64,
) -> Result<usize, toasty::Error> {
    Ok(User::all()
        .exec(db)
        .await?
        .iter()
        .filter(|user| {
            user.id != except && user.role == Role::Admin && user.status == UserStatus::Active
        })
        .count())
}

/// Whether taking `user` out of the active admins leaves none.
async fn is_last_admin(db: &mut dyn toasty::Executor, user: &User) -> Result<bool, toasty::Error> {
    Ok(user.role == Role::Admin
        && user.status == UserStatus::Active
        && other_active_admins(db, user.id).await? == 0)
}

async fn load(db: &mut dyn toasty::Executor, id: u64) -> Result<User, AccountError> {
    User::filter_by_id(id)
        .first()
        .exec(db)
        .await?
        .ok_or(AccountError::NotFound)
}

/// Makes a user an admin, or not.
///
/// # Errors
///
/// Fails for the last active admin, unknown users, or on database errors.
pub async fn set_role(db: &mut Db, id: u64, role: Role) -> Result<User, AccountError> {
    let mut tx = pixiu_db::write_transaction(db).await?;
    let mut user = load(&mut tx, id).await?;
    if role != Role::Admin && is_last_admin(&mut tx, &user).await? {
        return Err(AccountError::LastAdmin);
    }
    toasty::update!(user { role }).exec(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(user = id, ?role, "role changed");
    Ok(user)
}

/// Turns an account on or off (or moves it along registration).
///
/// # Errors
///
/// Fails for the last active admin, unknown users, or on database errors.
pub async fn set_status(db: &mut Db, id: u64, status: UserStatus) -> Result<User, AccountError> {
    let mut tx = pixiu_db::write_transaction(db).await?;
    let mut user = load(&mut tx, id).await?;
    if status != UserStatus::Active && is_last_admin(&mut tx, &user).await? {
        return Err(AccountError::LastAdmin);
    }
    toasty::update!(user { status }).exec(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(user = id, ?status, "status changed");
    Ok(user)
}

/// Whether the user may be deleted: anyone but the last active admin.
///
/// # Errors
///
/// Fails for unknown users, or on database errors.
pub async fn check_deletable(db: &mut dyn toasty::Executor, id: u64) -> Result<User, AccountError> {
    let user = load(db, id).await?;
    if is_last_admin(db, &user).await? {
        return Err(AccountError::LastAdmin);
    }
    Ok(user)
}

/// Deletes a user and everything in their library, in one transaction.
/// Stored files are left for [`pixiu_treasury`]'s garbage collection, which
/// keeps those other libraries still play.
///
/// # Errors
///
/// Fails for the last active admin, unknown users, or on database errors.
pub async fn delete(db: &mut Db, id: u64) -> Result<(), AccountError> {
    let mut tx = pixiu_db::write_transaction(db).await?;
    check_deletable(&mut tx, id).await?;
    let owner = i64::try_from(id).unwrap_or(i64::MAX);
    for statement in [
        "DELETE FROM track_claims WHERE track_id IN (SELECT id FROM tracks WHERE user_id = ?1)",
        "DELETE FROM released_claims WHERE track_id IN (SELECT id FROM tracks WHERE user_id = ?1)",
        "DELETE FROM lyrics WHERE track_id IN (SELECT id FROM tracks WHERE user_id = ?1)",
        "DELETE FROM playlist_entries WHERE playlist_id IN \
         (SELECT id FROM playlists WHERE user_id = ?1)",
        "DELETE FROM watch_exclusions WHERE watch_id IN (SELECT id FROM watches WHERE user_id = ?1)",
        "DELETE FROM tracks WHERE user_id = ?1",
        "DELETE FROM albums WHERE user_id = ?1",
        "DELETE FROM artists WHERE user_id = ?1",
        "DELETE FROM annotations WHERE user_id = ?1",
        "DELETE FROM playlists WHERE user_id = ?1",
        "DELETE FROM playlist_folders WHERE user_id = ?1",
        "DELETE FROM watches WHERE user_id = ?1",
        "DELETE FROM jobs WHERE user_id = ?1",
        "DELETE FROM offerings WHERE user_id = ?1",
        "DELETE FROM play_queues WHERE user_id = ?1",
        "DELETE FROM source_sessions WHERE user_id = ?1",
        "DELETE FROM session_events WHERE user_id = ?1",
        "DELETE FROM api_keys WHERE user_id = ?1",
        "DELETE FROM web_sessions WHERE user_id = ?1",
        "DELETE FROM account_tokens WHERE user_id = ?1",
        "DELETE FROM user_settings WHERE user_id = ?1",
        "DELETE FROM sent_alerts WHERE user_id = ?1",
        "DELETE FROM users WHERE id = ?1",
    ] {
        toasty::sql::query(statement)
            .bind(owner)
            .exec(&mut tx)
            .await?;
    }
    tx.commit().await?;
    tracing::info!(user = id, "account deleted with its library");
    Ok(())
}

/// What a library takes up.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub songs: u64,
    /// The stored files the library plays.
    pub bytes: u64,
    /// Of those, the files no other library plays: what deleting the user
    /// frees.
    pub exclusive_bytes: u64,
}

/// Every library's usage, by user.
///
/// # Errors
///
/// Fails on database errors.
pub async fn usage(db: &mut Db) -> Result<HashMap<u64, Usage>, toasty::Error> {
    let mut usage: HashMap<u64, Usage> = HashMap::new();
    let rows = toasty::sql::query(
        "SELECT tracks.user_id, COUNT(*), COALESCE(SUM(audio_files.size), 0) FROM tracks \
         LEFT JOIN audio_files ON audio_files.id = tracks.file_id GROUP BY tracks.user_id",
    )
    .exec(db)
    .await?;
    for row in rows {
        if let toasty::stmt::Value::Record(record) = row
            && let Some(owner) = as_u64(&record[0])
        {
            let entry = usage.entry(owner).or_default();
            entry.songs = as_u64(&record[1]).unwrap_or(0);
            entry.bytes = as_u64(&record[2]).unwrap_or(0);
        }
    }
    let rows = toasty::sql::query(
        "SELECT owner, SUM(size) FROM (\
            SELECT MIN(tracks.user_id) AS owner, audio_files.size AS size, \
            COUNT(DISTINCT tracks.user_id) AS owners \
            FROM audio_files JOIN tracks ON tracks.file_id = audio_files.id \
            GROUP BY audio_files.id\
         ) WHERE owners = 1 GROUP BY owner",
    )
    .exec(db)
    .await?;
    for row in rows {
        if let toasty::stmt::Value::Record(record) = row
            && let Some(owner) = as_u64(&record[0])
        {
            usage.entry(owner).or_default().exclusive_bytes = as_u64(&record[1]).unwrap_or(0);
        }
    }
    Ok(usage)
}

/// What the store holds, for every library together.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Store {
    pub files: u64,
    pub bytes: u64,
    /// Files more than one library plays.
    pub shared_files: u64,
}

/// What the store holds.
///
/// # Errors
///
/// Fails on database errors.
pub async fn store(db: &mut Db) -> Result<Store, toasty::Error> {
    let rows = toasty::sql::query(
        "SELECT COUNT(*), COALESCE(SUM(size), 0), \
         (SELECT COUNT(*) FROM (SELECT file_id FROM tracks GROUP BY file_id \
          HAVING COUNT(DISTINCT user_id) > 1)) FROM audio_files",
    )
    .exec(db)
    .await?;
    let number = |index: usize| match rows.first() {
        Some(toasty::stmt::Value::Record(record)) => as_u64(&record[index]).unwrap_or(0),
        _ => 0,
    };
    Ok(Store {
        files: number(0),
        bytes: number(1),
        shared_files: number(2),
    })
}
