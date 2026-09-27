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
    ui::{BUTTON_PRIMARY, alert, card, field},
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
        card(
            <form method="post" action=(LOGIN_PATH) class="flex flex-col gap-4">
                <h2 class="text-xl font-semibold">"Enter the hoard"</h2>
                if failed {
                    alert("Wrong username or password.")
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
                        name="password" type="password" autocomplete="current-password"
                        required=""
                    },
                )
                <button type="submit" class=(BUTTON_PRIMARY)>"Sign in"</button>
            </form>
        )
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
