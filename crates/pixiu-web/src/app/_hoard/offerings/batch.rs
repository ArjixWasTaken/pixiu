//! `/offerings/{batch}/...`: act on everything uploaded together.

use pixiu_db::{Album, OfferingStatus};
use pixiu_jobs::NewJob;
use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        path_param, route,
    },
};

use super::OFFERINGS_PATH;
use crate::auth::{db, jobs, offerings, require_user};

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
    let outcome = offerings.accept_batch(batch).await?;
    // Look the new albums up on MusicBrainz, and find their lyrics.
    for album_id in outcome.albums {
        if let Some(album) = Album::filter_by_id(album_id)
            .first()
            .exec(&mut db(cx))
            .await?
        {
            let title = format!("Look up {}", album.title);
            jobs(cx)
                .enqueue(NewJob::enrich(album_id, &title, None, false))
                .await?;
        }
    }
    let failures = outcome.failures;
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
