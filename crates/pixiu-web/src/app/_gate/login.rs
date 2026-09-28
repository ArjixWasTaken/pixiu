//! `/login`: signing in to the WebUI.

use pixiu_db::User;
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
    auth::{
        LOGIN_PATH, SETUP_PATH, current_user, db, is_claimed, remember_password, start_session,
        verify_password,
    },
    ui::{LOGO_SMALL, Size, Tone, alert, btn, text_field},
};

#[query_params(error = bad_request)]
struct LoginQuery {
    error: Option<String>,
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    if !is_claimed(cx).await? {
        return Err(redirect(SETUP_PATH).into());
    }
    if current_user(cx).await.map_err(Clone::clone)?.is_some() {
        return Err(redirect("/").into());
    }
    let failed = query_params::<LoginQuery>(cx)?.error.is_some();

    Ok(view! {
        <div
            class="flex min-h-dvh items-center justify-center px-5 py-8 \
                   bg-[radial-gradient(50%_40%_at_50%_20%,rgb(230_182_92/.10),transparent_70%)]"
        >
            <form
                method="post"
                action=(LOGIN_PATH)
                class="flex w-full max-w-[400px] flex-col gap-5"
            >
                <div class="flex flex-col items-center gap-3.5 text-center">
                    <div
                        class="grid size-28 place-items-center rounded-full border border-gold/40 bg-dim"
                    >
                        <img src=(LOGO_SMALL) alt="" width="88" height="88" class="size-[88px]">
                    </div>
                    <h1 class="m-0 text-[32px] leading-10 font-normal">"Enter the hoard"</h1>
                    <p class="m-0 text-sm text-muted-foreground">"Sign in as the keeper of this píxiū."</p>
                </div>
                text_field(
                    label: "Username",
                    attrs: attributes! {
                        name="username" autocomplete="username" required="" autofocus=""
                    },
                )
                text_field(
                    label: "Password",
                    attrs: attributes! {
                        name="password" type="password" autocomplete="current-password"
                        required=""
                    },
                )
                if failed {
                    alert("Wrong username or password.")
                }
                <button type="submit" class=(btn(Tone::Filled, Size::M))>"Sign in"</button>
            </form>
        </div>
    })
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

#[route(POST)]
async fn login(cx: &Cx, Form(form): Form<LoginForm>) -> Result<SeeOther> {
    let username = form.username.trim();
    let user = User::filter_by_username(username)
        .first()
        .exec(&mut db(cx))
        .await?;
    let hash = user.as_ref().map(|user| user.password_hash.clone());
    // Always verify, even for unknown users; see `verify_password`.
    let verified = verify_password(form.password.clone(), hash).await?;

    match user {
        Some(mut user) if verified => {
            if user.subsonic_secret.is_none() {
                remember_password(cx, &mut user, &form.password).await?;
            }
            start_session(cx, &user).await?;
            Ok(see_other("/"))
        }
        _ => {
            tracing::warn!(username, "failed WebUI login");
            Ok(see_other(format!("{LOGIN_PATH}?error=credentials")))
        }
    }
}
