//! `/settings/api-keys/{key_id}/...`.

use pixiu_db::ApiKey;
use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        path_param, route,
    },
};

use super::super::SETTINGS_PATH;
use crate::auth::{db, require_user};

path_param!(key_id: u64, error = bad_request);

#[route(POST "./revoke")]
async fn revoke(cx: &Cx) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let id = *path_param::<KeyId>(cx)?;
    let mut db = db(cx);
    if let Some(key) = ApiKey::filter_by_id(id).first().exec(&mut db).await?
        && key.user_id == user.id
    {
        key.delete().exec(&mut db).await?;
        tracing::info!(id, "API key revoked");
    }
    Ok(see_other(SETTINGS_PATH))
}
