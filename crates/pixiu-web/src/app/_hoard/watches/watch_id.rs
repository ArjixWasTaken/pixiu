//! `/watches/{watch_id}/...`.

use pixiu_db::Watch;
use pixiu_jobs::watch;
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
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

/// A song of a watched playlist, and the page to go back to.
#[derive(Deserialize)]
struct SongForm {
    video: String,
    back: Option<String>,
}

impl SongForm {
    /// Where to go afterwards: a path on this site, else the playlists.
    fn back(&self) -> SeeOther {
        match self.back.as_deref() {
            Some(back) if back.starts_with('/') && !back.starts_with("//") => see_other(back),
            _ => see_other("/playlists"),
        }
    }
}

/// Excludes a song from the watched playlist: it leaves the mirror, the
/// watch no longer keeps or fetches it.
#[route(POST "./exclude")]
async fn exclude(cx: &Cx, Form(form): Form<SongForm>) -> Result<SeeOther> {
    require_user(cx).await?;
    let id = *path_param::<WatchId>(cx)?;
    watch::exclude(treasury(cx), jobs(cx), id, &form.video).await?;
    Ok(form.back())
}

/// Takes an exclusion back.
#[route(POST "./include")]
async fn include(cx: &Cx, Form(form): Form<SongForm>) -> Result<SeeOther> {
    require_user(cx).await?;
    let id = *path_param::<WatchId>(cx)?;
    watch::include(treasury(cx), jobs(cx), id, &form.video).await?;
    Ok(form.back())
}
