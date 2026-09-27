//! `GET /covers/{album_id}`: an album cover for the WebUI (Subsonic clients
//! use `getCoverArt`).

use pixiu_db::Album;
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, HeaderValue, error::RouterErrorExt, header, path_param, query_params,
        response::Response, route,
    },
};

use crate::auth::{db, require_user, treasury};

path_param!(album_id: u64, error = bad_request);

#[query_params(error = bad_request)]
struct CoverQuery {
    size: Option<u32>,
}

#[route(GET)]
async fn cover(cx: &Cx) -> Result<Response> {
    require_user(cx).await?;
    let id = *path_param::<AlbumId>(cx)?;
    let album = Album::filter_by_id(id)
        .first()
        .exec(&mut db(cx))
        .await?
        .ok_or_not_found()?;
    let cover = album.cover.ok_or_not_found()?;

    let treasury = treasury(cx);
    let size = query_params::<CoverQuery>(cx)?.size;
    let path = pixiu_treasury::covers::sized(&treasury.resolve(&cover), treasury.cache_dir(), size)
        .await?;
    let bytes = tokio::fs::read(&path).await?;
    let content_type = match path.extension().and_then(|extension| extension.to_str()) {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "image/jpeg",
    };

    let mut response = Response::new(Body::from(bytes));
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=86400"),
    );
    Ok(response)
}
