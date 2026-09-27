//! `/settings/api-keys`: creating and revoking OpenSubsonic API keys.

mod key_id;

use pixiu_db::{ApiKey, now, toasty};
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{content::Form, page},
    view::{View, view},
};

use super::settings;
use crate::auth::{db, require_user};

#[derive(Deserialize)]
struct NewKey {
    name: String,
}

/// Creates a key and shows it once, in the response to this POST (a
/// redirect would have to carry the key in the URL).
#[page(POST)]
async fn create(cx: &Cx, Form(form): Form<NewKey>) -> Result<impl View> {
    let user = require_user(cx).await?;
    let name = match form.name.trim() {
        "" => "Unnamed".to_owned(),
        name => name.to_owned(),
    };

    let key = format!("pixiu_{}", hex::encode(rand::random::<[u8; 24]>()));
    toasty::create!(ApiKey {
        user_id: user.id,
        name: &name,
        key_hash: ApiKey::hash(&key),
        created_at: now(),
    })
    .exec(&mut db(cx))
    .await?;
    tracing::info!(name, "API key created");

    Ok(view! { settings(created_name: Some(name.as_str()), created_key: Some(key.as_str())) })
}
