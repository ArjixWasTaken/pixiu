//! `/offerings/{batch}/...`: act on everything uploaded together.

use pixiu_db::OfferingStatus;
use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        path_param, route,
    },
};

use super::OFFERINGS_PATH;
use crate::auth::{offerings, require_user};

path_param!(batch);

/// Absorbs every readable offering of the batch.
#[route(POST "./accept")]
async fn accept(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    let batch = path_param::<Batch>(cx);
    let offerings = offerings(cx);
    let total = offerings
        .pending()
        .await?
        .iter()
        .filter(|item| item.batch == batch && item.status == OfferingStatus::Pending)
        .count();
    let failures = offerings.accept_batch(batch).await?;
    let accepted = total - failures.len();
    Ok(see_other(if failures.is_empty() {
        format!("{OFFERINGS_PATH}?accepted={accepted}")
    } else {
        format!(
            "{OFFERINGS_PATH}?accepted={accepted}&failed={}",
            failures.len()
        )
    }))
}

#[route(POST "./discard")]
async fn discard(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    offerings(cx).discard_batch(path_param::<Batch>(cx)).await?;
    Ok(see_other(OFFERINGS_PATH))
}
