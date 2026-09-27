//! `/offerings/item/{offering_id}/...`.

use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        path_param, route,
    },
};

use super::super::OFFERINGS_PATH;
use crate::auth::{offerings, require_user};

path_param!(offering_id: u64, error = bad_request);

#[route(POST "./discard")]
async fn discard(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    offerings(cx)
        .discard(*path_param::<OfferingId>(cx)?)
        .await?;
    Ok(see_other(OFFERINGS_PATH))
}
