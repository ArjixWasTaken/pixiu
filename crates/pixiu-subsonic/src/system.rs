//! System endpoints: license, extensions, scanning and users.

use pixiu_db::{Role, User, owned::as_u64};

use crate::{
    Cx, Failure, Params,
    browse::MUSIC_FOLDER_ID,
    response::{ApiError, Element, ErrorCode, Payload},
};

/// OpenSubsonic extensions and the versions of each that píxiū implements.
const EXTENSIONS: &[(&str, &[u32])] = &[
    ("formPost", &[1]),
    ("apiKeyAuthentication", &[1]),
    ("songLyrics", &[1]),
    ("transcodeOffset", &[1]),
];

pub(crate) fn extensions() -> Payload {
    Payload::list(
        "openSubsonicExtensions",
        EXTENSIONS.iter().map(|(name, versions)| {
            Element::new("openSubsonicExtensions")
                .attr("name", *name)
                .values("versions", versions.iter().copied())
        }),
    )
}

pub(crate) fn license() -> Payload {
    Element::new("license").attr("valid", true).into()
}

/// `getScanStatus` and `startScan`. píxiū never scans: everything enters
/// through ingest, so the library is always up to date. The count is the
/// caller's songs.
pub(crate) async fn scan_status(cx: &Cx<'_>) -> Result<Payload, Failure> {
    let rows = cx
        .lib
        .sql("SELECT COUNT(*) FROM tracks WHERE tracks.user_id = ?1")
        .rows(&mut cx.lib.db())
        .await?;
    let count = rows
        .first()
        .and_then(|row| row.first())
        .and_then(as_u64)
        .unwrap_or(0);
    Ok(Element::new("scanStatus")
        .attr("scanning", false)
        .attr("count", count)
        .into())
}

/// `getUser`: admins see anyone; others, themselves.
pub(crate) async fn user(cx: &Cx<'_>, params: &Params) -> Result<Payload, Failure> {
    let username = params.require("username")?;
    if username == cx.user.username {
        return Ok(user_element(cx.user).into());
    }
    if cx.user.role != Role::Admin {
        return Err(ApiError::new(ErrorCode::NotAuthorized, "no such user").into());
    }
    let other = User::filter_by_username(username)
        .first()
        .exec(&mut cx.lib.db())
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "no such user"))?;
    Ok(user_element(&other).into())
}

/// `getUsers`: every account, for admins.
pub(crate) async fn users(cx: &Cx<'_>) -> Result<Payload, Failure> {
    if cx.user.role != Role::Admin {
        return Err(ApiError::new(ErrorCode::NotAuthorized, "only admins list users").into());
    }
    let mut accounts = User::all().exec(&mut cx.lib.db()).await?;
    accounts.sort_by_key(|user| user.id);
    Ok(Element::new("users")
        .list("user", accounts.iter().map(user_element))
        .into())
}

/// `tokenInfo` (OpenSubsonic API keys): whose key it is.
pub(crate) fn token_info(user: &User) -> Payload {
    Element::new("tokenInfo")
        .attr("username", user.username.as_str())
        .into()
}

fn user_element(user: &User) -> Element {
    Element::new("user")
        .attr("username", user.username.as_str())
        .attr_opt("email", user.email.as_deref())
        .attr("scrobblingEnabled", false)
        .attr("adminRole", user.role == Role::Admin)
        .attr("settingsRole", true)
        .attr("downloadRole", true)
        .attr("uploadRole", false)
        .attr("playlistRole", true)
        .attr("coverArtRole", true)
        .attr("commentRole", false)
        .attr("podcastRole", false)
        .attr("streamRole", true)
        .attr("jukeboxRole", false)
        .attr("shareRole", false)
        .attr("videoConversionRole", false)
        .values("folder", [MUSIC_FOLDER_ID])
}

/// Read endpoints of features píxiū does not have (podcasts, radio,
/// bookmarks, shares). Their lists are truthfully empty, and answering keeps
/// clients that load them on startup from showing errors.
pub(crate) fn empty(method: &str) -> Option<Payload> {
    let (root, list) = match method {
        "getInternetRadioStations" => ("internetRadioStations", "internetRadioStation"),
        "getPodcasts" => ("podcasts", "channel"),
        "getNewestPodcasts" => ("newestPodcasts", "episode"),
        "getBookmarks" => ("bookmarks", "bookmark"),
        "getShares" => ("shares", "share"),
        _ => return None,
    };
    Some(Element::new(root).list(list, Vec::new()).into())
}
