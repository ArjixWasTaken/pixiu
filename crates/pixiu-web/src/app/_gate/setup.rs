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
    ui::{LABEL, LOGO, Size, Tone, alert, btn, text_field},
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
        <div class="flex min-h-dvh flex-wrap">
            <div
                class="flex min-h-80 flex-[1_1_360px] items-center justify-center p-10 \
                       bg-[radial-gradient(60%_60%_at_50%_45%,rgb(230_182_92/.12),transparent_70%)]"
            >
                <div class="relative grid aspect-square w-[min(340px,70vw)] place-items-center">
                    <div class="absolute inset-0 rounded-full border border-gold/35"></div>
                    <div class="absolute inset-[18px] rounded-full border border-dashed border-gold/20"></div>
                    <img src=(LOGO) alt="píxiū" class="h-auto w-[78%]">
                </div>
            </div>
            <div class="flex flex-[1_1_360px] items-center justify-center px-6 py-10">
                <form
                    method="post"
                    action=(SETUP_PATH)
                    class="flex w-full max-w-[380px] flex-col gap-5"
                >
                    <div class="flex flex-col gap-2">
                        <span class=(LABEL)>"First run"</span>
                        <h1 class="m-0 text-[40px] leading-11 font-normal">"Claim this hoard"</h1>
                        <p class="m-0 text-[15px] leading-[22px] text-pretty text-muted-foreground">
                            "There's no keeper yet. The account you make here is the only one, "
                            "and it also signs in your Subsonic apps."
                        </p>
                    </div>
                    text_field(
                        label: "Username",
                        invalid: problem == Some(Problem::EmptyUsername),
                        attrs: attributes! {
                            name="username" autocomplete="username" required="" autofocus=""
                        },
                    )
                    text_field(
                        label: "Password",
                        supporting: "At least 8 characters",
                        invalid: problem == Some(Problem::ShortPassword),
                        attrs: attributes! {
                            name="password" type="password" autocomplete="new-password"
                            required="" minlength="8"
                        },
                    )
                    text_field(
                        label: "Confirm password",
                        invalid: problem == Some(Problem::Mismatch),
                        attrs: attributes! {
                            name="confirm" type="password" autocomplete="new-password" required=""
                        },
                    )
                    if let Some(problem) = problem {
                        alert((problem.message()))
                    }
                    <button type="submit" class=(btn(Tone::Filled, Size::M))>"Claim the hoard"</button>
                </form>
            </div>
        </div>
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
