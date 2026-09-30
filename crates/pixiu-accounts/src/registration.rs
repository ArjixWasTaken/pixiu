//! Open registration: anyone may ask for an account while admins allow it
//! and email works. Each request waits for an admin; approved, the
//! applicant confirms their address and can sign in; denied, they get a
//! short note and the request is gone.

use pixiu_core::SecretBox;
use pixiu_db::{Db, Role, User, UserStatus, toasty};

use crate::{
    AccountError, Mailer, NewUser, links,
    mail::templates,
    users::{self, check_email},
};

/// What someone asks for.
pub struct Request {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// How a request went. The applicant hears the same either way.
#[derive(Debug)]
pub enum Requested {
    /// A new request, waiting for an admin.
    Waiting(User),
    /// The address has an account already; its owner got a note.
    EmailInUse,
}

/// Asks for an account.
///
/// # Errors
///
/// Fails when registration is closed, a value breaks a rule, the username
/// is taken, or on database errors.
pub async fn request(
    db: &mut Db,
    secrets: &SecretBox,
    mailer: &Mailer,
    request: Request,
) -> Result<Requested, AccountError> {
    let settings = mailer.settings().get();
    if !settings.registration_available() {
        return Err(AccountError::RegistrationClosed);
    }
    let email = check_email(&request.email)?.ok_or_else(|| {
        AccountError::Invalid(
            "Your email address, please: píxiū tells you there how it went.".to_owned(),
        )
    })?;
    let created = users::create(
        db,
        secrets,
        NewUser {
            username: request.username,
            email: Some(email.clone()),
            email_verified: false,
            password: request.password,
            role: Role::User,
            status: UserStatus::Pending,
            password_change_required: false,
        },
    )
    .await;
    let user = match created {
        Ok(user) => user,
        Err(AccountError::EmailTaken) => {
            let link = settings.link("/").unwrap_or_default();
            mailer.queue(templates::email_in_use(&email, &link));
            tracing::info!("registration with an address in use; its owner was told");
            return Ok(Requested::EmailInUse);
        }
        Err(error) => return Err(error),
    };
    let link = settings.link("/settings?tab=users").unwrap_or_default();
    for admin in User::all().exec(db).await? {
        if let (Role::Admin, UserStatus::Active, Some(to), Some(_)) = (
            admin.role,
            admin.status,
            &admin.email,
            admin.email_verified_at,
        ) {
            mailer.queue(templates::new_registration(
                to,
                &user.username,
                &email,
                &link,
            ));
        }
    }
    tracing::info!(user = user.id, username = %user.username, "registration requested");
    Ok(Requested::Waiting(user))
}

/// A request, if `id` is one.
async fn pending(db: &mut Db, id: u64) -> Result<User, AccountError> {
    let user = User::filter_by_id(id)
        .first()
        .exec(db)
        .await?
        .ok_or(AccountError::NotFound)?;
    if user.status != UserStatus::Pending {
        return Err(AccountError::NotPending);
    }
    Ok(user)
}

/// Approves a request: the applicant gets the link confirming their
/// address, which opens their account.
///
/// # Errors
///
/// Fails when it is not a waiting request, email does not work, or on
/// database errors.
pub async fn approve(db: &mut Db, mailer: &Mailer, id: u64) -> Result<User, AccountError> {
    pending(db, id).await?;
    if !mailer.ready() {
        return Err(AccountError::Invalid(
            "Email is not set up on this server, so the applicant could not confirm their address."
                .to_owned(),
        ));
    }
    let user = users::set_status(db, id, UserStatus::Unverified).await?;
    links::send_verification(db, mailer, &user).await?;
    tracing::info!(user = id, "registration approved");
    Ok(user)
}

/// Denies a request: the applicant gets a short note, and the request goes.
///
/// # Errors
///
/// Fails when it is not a waiting request, or on database errors.
pub async fn deny(db: &mut Db, mailer: &Mailer, id: u64) -> Result<(), AccountError> {
    let user = pending(db, id).await?;
    users::delete(db, id).await?;
    if let (Some(to), true) = (&user.email, mailer.ready()) {
        mailer.queue(templates::registration_declined(to, &user.username));
    }
    tracing::info!(user = id, "registration denied");
    Ok(())
}

/// How many requests wait for an admin.
///
/// # Errors
///
/// Fails on database errors.
pub async fn waiting(db: &mut Db) -> Result<usize, toasty::Error> {
    Ok(User::all()
        .exec(db)
        .await?
        .iter()
        .filter(|user| user.status == UserStatus::Pending)
        .count())
}
