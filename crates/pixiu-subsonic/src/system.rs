//! System endpoints: license, extensions, scanning and users.

use pixiu_db::{User, toasty};

use crate::{
    Failure, Params, SubsonicState,
    browse::MUSIC_FOLDER_ID,
    catalog,
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
/// through ingest, so the library is always up to date.
pub(crate) async fn scan_status(state: &SubsonicState) -> Result<Payload, Failure> {
    let rows = toasty::sql::query("SELECT COUNT(*) FROM tracks")
        .exec(&mut state.db.clone())
        .await?;
    let count = rows
        .first()
        .and_then(|row| match row {
            toasty::stmt::Value::Record(record) => catalog::as_u64(&record[0]),
            _ => None,
        })
        .unwrap_or(0);
    Ok(Element::new("scanStatus")
        .attr("scanning", false)
        .attr("count", count)
        .into())
}

/// `getUser`. Only the admin exists.
pub(crate) fn user(user: &User, params: &Params) -> Result<Payload, Failure> {
    let username = params.require("username")?;
    if username != user.username {
        return Err(ApiError::new(ErrorCode::NotAuthorized, "no such user").into());
    }
    Ok(user_element(user).into())
}

/// `getUsers`: píxiū has one.
pub(crate) fn users(user: &User) -> Payload {
    Element::new("users")
        .list("user", [user_element(user)])
        .into()
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
        .attr("scrobblingEnabled", false)
        .attr("adminRole", true)
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
