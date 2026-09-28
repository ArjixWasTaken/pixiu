//! `/watches/{watch_id}/...`.

use pixiu_db::Watch;
use pixiu_jobs::watch;
use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        path_param, route,
    },
};

use super::WATCHES_PATH;
use crate::auth::{db, jobs, require_user, treasury};

path_param!(watch_id: u64, error = bad_request);

#[route(POST "./sync")]
async fn sync(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    let id = *path_param::<WatchId>(cx)?;
    if let Some(watch) = Watch::filter_by_id(id).first().exec(&mut db(cx)).await? {
        watch::queue_sync(jobs(cx), &watch).await?;
    }
    // The watch list follows the sync live.
    Ok(see_other(WATCHES_PATH))
}

#[route(POST "./remove")]
async fn remove(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    watch::remove(treasury(cx), jobs(cx), *path_param::<WatchId>(cx)?).await?;
    Ok(see_other(format!("{WATCHES_PATH}?removed=1")))
}
