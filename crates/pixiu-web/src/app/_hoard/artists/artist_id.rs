//! `/artists/{artist_id}`: an artist, what the hoard holds of theirs, and
//! their picture (`./image`).

use pixiu_db::{Album, Artist, Watch, WatchKind};
use topcoat::{
    Result,
    context::Cx,
    icon::icon,
    router::{
        Body, HeaderValue, error::RouterErrorExt, header, page, path_param, response::Response,
        route,
    },
    view::{View, view},
};

use crate::{
    app::_hoard::count,
    auth::{db, require_user, treasury},
    ui::{ALBUM_GRID, EYEBROW, H2, Size, Tone, album_tile, artist_picture, btn, icons},
};

path_param!(artist_id: u64, error = bad_request);

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let id = *path_param::<ArtistId>(cx)?;
    let mut db = db(cx);
    let artist = Artist::filter_by_id(id)
        .first()
        .exec(&mut db)
        .await?
        .ok_or_not_found()?;
    let mut albums = Album::filter_by_artist_id(id).exec(&mut db).await?;
    albums.sort_by_key(|album| (std::cmp::Reverse(album.year), album.title.to_lowercase()));
    let watched = match &artist.ytm_channel_id {
        Some(channel) => Watch::filter_by_remote_id(channel)
            .first()
            .exec(&mut db)
            .await?
            .is_some_and(|watch| watch.kind == WatchKind::Artist),
        None => false,
    };
    let watch_link = artist.ytm_channel_id.as_ref().map(|channel| {
        let link = format!("https://music.youtube.com/channel/{channel}");
        format!(
            "/watches?target={}",
            form_urlencoded::byte_serialize(link.as_bytes()).collect::<String>()
        )
    });

    Ok(view! {
        <div class="flex flex-col gap-8">
            <section class="flex flex-wrap items-center gap-7">
                artist_picture(artist: &artist, size: "size-[clamp(120px,22vw,180px)] text-[56px]")
                <div class="flex min-w-0 flex-[1_1_300px] flex-col gap-2.5">
                    <span class=(EYEBROW)>
                        "Artist · " (count(albums.len(), "album in the hoard", "albums in the hoard"))
                    </span>
                    <h1 class="m-0 text-[clamp(32px,5vw,52px)] leading-[1.05] font-normal">(&artist.name)</h1>
                    if let Some(bio) = &artist.bio {
                        <p class="m-0 max-w-[62ch] text-[15px] leading-[23px] text-pretty text-foreground-soft">
                            (bio)
                        </p>
                    } else if artist.info_fetched_at.is_some() {
                        <p class="m-0 text-[15px] text-muted-foreground">
                            "Wikipedia has no article about this artist yet."
                        </p>
                    }
                    <div class="mt-1 flex flex-wrap items-center gap-3">
                        if watched {
                            <a
                                href="/watches"
                                class="inline-flex h-8 items-center gap-1.5 rounded-lg bg-slate-container px-3 \
                                       text-sm text-foreground hover:text-foreground"
                            >
                                icon(data: icons::CHECK, size: 18)
                                "Watched"
                            </a>
                        } else if let Some(link) = &watch_link {
                            <a href=(link) class=(btn(Tone::Tonal, Size::S))>"Watch this artist"</a>
                        }
                        if let Some(url) = &artist.bio_url {
                            <a href=(url) target="_blank" rel="noopener" class="text-sm">"Read on Wikipedia"</a>
                        }
                    </div>
                </div>
            </section>
            <section class="flex flex-col gap-4">
                <h2 class=(H2)>"In the hoard"</h2>
                <div class=(ALBUM_GRID)>
                    for album in &albums {
                        let href = format!("/albums/{}", album.id);
                        album_tile(
                            href: &href,
                            cover_of: album.cover.as_ref().map(|_| album.id),
                            title: &album.title,
                            subtitle: album.year.map(|year| year.to_string()).unwrap_or_default(),
                        )
                    }
                </div>
            </section>
        </div>
    })
}

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
