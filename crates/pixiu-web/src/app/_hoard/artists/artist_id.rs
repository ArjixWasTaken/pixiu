//! `GET /artists/{artist_id}/image`: an artist's picture.

use pixiu_db::Artist;
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, HeaderValue, error::RouterErrorExt, header, path_param, response::Response, route,
    },
};

use crate::auth::{db, require_user, treasury};

path_param!(artist_id: u64, error = bad_request);

#[route(GET "./image")]
async fn image(cx: &Cx) -> Result<Response> {
    require_user(cx).await?;
    let id = *path_param::<ArtistId>(cx)?;
    let image = Artist::filter_by_id(id)
        .first()
        .exec(&mut db(cx))
        .await?
        .and_then(|artist| artist.image)
        .ok_or_not_found()?;
    let treasury = treasury(cx);
    let path = pixiu_treasury::covers::sized(
        &treasury.cache_dir().join(image),
        treasury.cache_dir(),
        Some(300),
    )
    .await?;
    let bytes = tokio::fs::read(&path).await?;
    let content_type =
        pixiu_treasury::covers::mime_of(&bytes).unwrap_or("application/octet-stream");
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=86400"),
    );
    Ok(response)
}
