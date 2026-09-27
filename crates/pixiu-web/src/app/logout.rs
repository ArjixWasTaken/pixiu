//! `POST /logout`.

use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        route,
    },
};

use crate::auth::{LOGIN_PATH, end_session};

#[route(POST)]
async fn logout(cx: &Cx) -> Result<SeeOther> {
    end_session(cx).await?;
    Ok(see_other(LOGIN_PATH))
}
