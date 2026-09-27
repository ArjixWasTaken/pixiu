//! `/setup`: first-run creation of the admin account.
//!
//! Whoever reaches a fresh instance first claims it. That is why the server
//! binds to loopback unless configured otherwise.

use pixiu_db::{User, now, toasty};
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, redirect, see_other},
        page, query_params, route,
    },
    view::{View, attributes, view},
};

use crate::{
    auth::{LOGIN_PATH, SETUP_PATH, db, hash_password, is_claimed, secrets, start_session},
    ui::{BUTTON_PRIMARY, alert, card, field},
};

const MIN_PASSWORD_LEN: usize = 8;

#[query_params(error = bad_request)]
struct SetupQuery {
    error: Option<String>,
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    if is_claimed(cx).await? {
        return Err(redirect(LOGIN_PATH).into());
    }
    let query = query_params::<SetupQuery>(cx)?;
    let problem = query.error.as_deref().and_then(Problem::from_code);

    Ok(view! {
        card(
            <form method="post" action=(SETUP_PATH) class="flex flex-col gap-4">
                <div class="flex flex-col gap-1">
                    <h2 class="text-xl font-semibold">"Claim this hoard"</h2>
                    <p class="text-sm text-muted-foreground">
                        "Create the admin account. The same credentials sign in "
                        "to the WebUI and Subsonic clients."
                    </p>
                </div>
                if let Some(problem) = problem {
                    alert((problem.message()))
                }
                field(
                    label: "Username",
                    attrs: attributes! {
                        name="username" autocomplete="username" required="" autofocus=""
                    },
                )
                field(
                    label: "Password",
                    attrs: attributes! {
                        name="password" type="password" autocomplete="new-password"
                        required="" minlength="8"
                    },
                )
                field(
                    label: "Confirm password",
                    attrs: attributes! {
                        name="confirm" type="password" autocomplete="new-password" required=""
                    },
                )
                <button type="submit" class=(BUTTON_PRIMARY)>"Claim"</button>
            </form>
        )
    })
}

#[derive(Deserialize)]
struct SetupForm {
    username: String,
    password: String,
    confirm: String,
}

#[route(POST)]
async fn claim(cx: &Cx, Form(form): Form<SetupForm>) -> Result<SeeOther> {
    if is_claimed(cx).await? {
        return Ok(see_other(LOGIN_PATH));
    }
    let username = form.username.trim().to_owned();
    if let Some(problem) = Problem::check(&username, &form.password, &form.confirm) {
        return Ok(see_other(format!("{SETUP_PATH}?error={}", problem.code())));
    }
    // Sealed for Subsonic token authentication, which needs the password.
    let subsonic_secret = secrets(cx).seal_str(&form.password);
    let password_hash = hash_password(form.password).await?;

    // The check and the insert share a transaction so two racing claims
    // cannot both create an admin.
    let mut db = db(cx);
    let mut tx = db.transaction().await?;
    if User::all().first().exec(&mut tx).await?.is_some() {
        return Ok(see_other(LOGIN_PATH));
    }
    let user = toasty::create!(User {
        username,
        password_hash,
        subsonic_secret: Some(subsonic_secret),
        created_at: now(),
    })
    .exec(&mut tx)
    .await?;
    tx.commit().await?;

    tracing::info!(username = %user.username, "admin account created");
    start_session(cx, &user).await?;
    Ok(see_other("/"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Problem {
    EmptyUsername,
    ShortPassword,
    Mismatch,
}

impl Problem {
    fn check(username: &str, password: &str, confirm: &str) -> Option<Self> {
        if username.is_empty() {
            Some(Self::EmptyUsername)
        } else if password.chars().count() < MIN_PASSWORD_LEN {
            Some(Self::ShortPassword)
        } else if password != confirm {
            Some(Self::Mismatch)
        } else {
            None
        }
    }

    fn code(self) -> &'static str {
        match self {
            Self::EmptyUsername => "username",
            Self::ShortPassword => "short",
            Self::Mismatch => "mismatch",
        }
    }

    fn from_code(code: &str) -> Option<Self> {
        [Self::EmptyUsername, Self::ShortPassword, Self::Mismatch]
            .into_iter()
            .find(|problem| problem.code() == code)
    }

    fn message(self) -> &'static str {
        match self {
            Self::EmptyUsername => "Choose a username.",
            Self::ShortPassword => "Use at least 8 characters for the password.",
            Self::Mismatch => "The passwords do not match.",
        }
    }
}
