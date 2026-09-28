//! `/jobs/{job_id}/...`.

use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        path_param, route,
    },
};

use super::JOBS_PATH;
use crate::auth::{jobs, require_user};

path_param!(job_id: u64, error = bad_request);

#[route(POST "./retry")]
async fn retry(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    jobs(cx).retry(*path_param::<JobId>(cx)?).await?;
    Ok(see_other(JOBS_PATH))
}
