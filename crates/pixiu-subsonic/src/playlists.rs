//! Playlists, each in its owner's library: made in Subsonic clients or the
//! web player; mirrors of watched YouTube Music playlists; and smart
//! playlists made in the web player, whose songs follow rules. Clients may
//! play mirrors and smart playlists but not change their songs.

use std::collections::{HashMap, HashSet};

use pixiu_db::{
    Album, ClaimKind, Db, Library, Playlist, PlaylistEntry, ReleaseReason, Track, User, now, toasty,
};
use pixiu_treasury::{Claim, Release};

use crate::{
    Cx, Failure, Params, catalog,
    ids::{self, Id},
    response::{ApiError, Element, ErrorCode, Payload},
};

fn not_found(what: &str) -> Failure {
    ApiError::new(ErrorCode::NotFound, format!("{what} not found")).into()
}

fn playlist_id(id: &str) -> Result<u64, Failure> {
    match Id::parse(id) {
        Some(Id::Playlist(id)) => Ok(id),
        _ => Err(not_found("playlist")),
    }
}

/// Why a playlist's songs cannot be changed here, if they cannot.
fn read_only(playlist: &Playlist) -> Option<Failure> {
    let reason = if playlist.watch_id.is_some() {
        "this playlist mirrors a watched playlist; change it on YouTube Music"
    } else if playlist.rules.is_some() {
        "this is a smart playlist; its songs follow its rules"
    } else {
        return None;
    };
    Some(ApiError::new(ErrorCode::NotAuthorized, reason).into())
}

async fn load(lib: &Library, id: u64) -> Result<Playlist, Failure> {
    lib.playlist(id).await?.ok_or_else(|| not_found("playlist"))
}

/// A playlist's tracks: their ids in order (a track may appear twice), and
/// the tracks. Mirrors list only what is downloaded; smart playlists, what
/// their rules match now.
struct Listing {
    order: Vec<u64>,
    tracks: HashMap<u64, Track>,
}

async fn listing(lib: &Library, playlist: &Playlist) -> Result<Listing, toasty::Error> {
    let db = &mut lib.db();
    if let Some(rules) = &playlist.rules {
        let order = crate::smart::track_ids(lib, rules).await?;
        let tracks = catalog::tracks_in_order(lib, &order)
            .await?
            .into_iter()
            .map(|track| (track.id, track))
            .collect();
        return Ok(Listing { order, tracks });
    }
    let mut entries = PlaylistEntry::filter_by_playlist_id(playlist.id)
        .exec(db)
        .await?;
    entries.sort_by_key(|entry| entry.position);

    let mut tracks: HashMap<u64, Track> = HashMap::new();
    let track_ids: Vec<u64> = entries.iter().filter_map(|entry| entry.track_id).collect();
    for track in catalog::tracks_in_order(lib, &track_ids).await? {
        tracks.insert(track.id, track);
    }
    let video_ids: Vec<String> = entries
        .iter()
        .filter_map(|entry| entry.ytm_video_id.clone())
        .collect();
    let mut by_video: HashMap<String, u64> = HashMap::new();
    for track in lib.tracks_of_videos(&video_ids).await? {
        if let Some(video_id) = &track.ytm_video_id {
            by_video.insert(video_id.clone(), track.id);
        }
        tracks.insert(track.id, track);
    }
    let order = entries
        .iter()
        .filter_map(|entry| match (&entry.track_id, &entry.ytm_video_id) {
            (Some(id), _) => tracks.contains_key(id).then_some(*id),
            (None, Some(video_id)) => by_video.get(video_id).copied(),
            (None, None) => None,
        })
        .collect();
    Ok(Listing { order, tracks })
}

/// The album whose cover stands for one of the library's playlists.
pub(crate) async fn cover_album(
    lib: &Library,
    playlist_id: u64,
) -> Result<Option<Album>, toasty::Error> {
    let Some(playlist) = lib.playlist(playlist_id).await? else {
        return Ok(None);
    };
    let listing = listing(lib, &playlist).await?;
    cover_of(lib, &listing).await
}

async fn cover_of(lib: &Library, listing: &Listing) -> Result<Option<Album>, toasty::Error> {
    for id in &listing.order {
        if let Some(album) = lib.album(listing.tracks[id].album_id).await?
            && album.cover.is_some()
        {
            return Ok(Some(album));
        }
    }
    Ok(None)
}

/// A playlist of `owner`'s library.
pub(crate) async fn describe(
    lib: &Library,
    playlist: &Playlist,
    owner: &User,
    with_entries: bool,
) -> Result<Element, toasty::Error> {
    let listing = listing(lib, playlist).await?;
    let duration: u64 = listing
        .order
        .iter()
        .map(|id| (listing.tracks[id].duration_ms + 500) / 1000)
        .sum();
    let cover = cover_of(lib, &listing).await?;
    let mut element = Element::new("playlist")
        .attr("id", ids::playlist(playlist.id))
        .attr("name", playlist.name.as_str())
        .attr_opt("comment", playlist.comment.as_deref())
        .attr("owner", owner.username.as_str())
        .attr("public", playlist.public)
        .attr("songCount", listing.order.len() as u64)
        .attr("duration", duration)
        .attr("created", playlist.created_at.to_string())
        .attr("changed", playlist.changed_at.to_string())
        .attr_opt("coverArt", cover.map(|album| ids::album(album.id)))
        .attr("readonly", read_only(playlist).is_some());
    if with_entries {
        // Each track is rendered once, then repeated where it recurs.
        let unique: Vec<Track> = listing.tracks.into_values().collect();
        let songs: HashMap<u64, Element> = unique
            .iter()
            .map(|track| track.id)
            .zip(catalog::songs(lib, "entry", &unique).await?)
            .collect();
        element = element.list("entry", listing.order.iter().map(|id| songs[id].clone()));
    }
    Ok(element)
}

/// `getPlaylists`: the caller's.
pub(crate) async fn list(cx: &Cx<'_>) -> Result<Payload, Failure> {
    let mut playlists = cx.lib.all_playlists().await?;
    playlists.sort_by_key(|playlist| playlist.name.to_lowercase());
    let mut elements = Vec::with_capacity(playlists.len());
    for playlist in &playlists {
        elements.push(describe(&cx.lib, playlist, cx.user, false).await?);
    }
    Ok(Element::new("playlists").list("playlist", elements).into())
}

pub(crate) async fn get(cx: &Cx<'_>, params: &Params) -> Result<Payload, Failure> {
    let playlist = load(&cx.lib, playlist_id(params.require("id")?)?).await?;
    Ok(describe(&cx.lib, &playlist, cx.user, true).await?.into())
}

/// The library's tracks from `songId`-style parameters, in order.
async fn requested_tracks<'a>(
    lib: &Library,
    ids: impl Iterator<Item = &'a str>,
) -> Result<Vec<u64>, Failure> {
    let mut track_ids = Vec::new();
    for id in ids {
        match Id::parse(id) {
            Some(Id::Track(id)) => track_ids.push(id),
            _ => return Err(not_found(&format!("song `{id}`"))),
        }
    }
    let found: HashSet<u64> = catalog::tracks_in_order(lib, &track_ids)
        .await?
        .into_iter()
        .map(|track| track.id)
        .collect();
    if let Some(missing) = track_ids.iter().find(|id| !found.contains(id)) {
        return Err(not_found(&format!("song `{}`", ids::track(*missing))));
    }
    Ok(track_ids)
}

/// Replaces a playlist's tracks (the library's), and has the playlist
/// claim them.
async fn set_tracks(
    cx: &Cx<'_>,
    db: &mut Db,
    playlist_id: u64,
    track_ids: &[u64],
) -> Result<(), Failure> {
    let mut tx = db.transaction().await?;
    PlaylistEntry::filter_by_playlist_id(playlist_id)
        .delete()
        .exec(&mut tx)
        .await?;
    for (position, track_id) in track_ids.iter().enumerate() {
        toasty::create!(PlaylistEntry {
            playlist_id,
            position: u32::try_from(position).unwrap_or(u32::MAX),
            track_id: Some(*track_id),
        })
        .exec(&mut tx)
        .await?;
    }
    tx.commit().await?;

    let reference = playlist_id.to_string();
    let listed: HashSet<u64> = track_ids.iter().copied().collect();
    let name = cx
        .lib
        .playlist(playlist_id)
        .await?
        .map(|playlist| playlist.name);
    let why = Release {
        reason: ReleaseReason::PlaylistEdited,
        source_name: name.as_deref(),
    };
    cx.state
        .treasury
        .release(
            cx.lib.owner(),
            ClaimKind::LocalPlaylist,
            &reference,
            why,
            |track| listed.contains(&track.id),
        )
        .await?;
    let claim = Claim {
        kind: ClaimKind::LocalPlaylist,
        reference: Some(reference),
    };
    for track_id in listed {
        cx.state.treasury.claim(track_id, &claim).await?;
    }
    Ok(())
}

async fn current_tracks(db: &mut Db, playlist_id: u64) -> Result<Vec<u64>, toasty::Error> {
    let mut entries = PlaylistEntry::filter_by_playlist_id(playlist_id)
        .exec(db)
        .await?;
    entries.sort_by_key(|entry| entry.position);
    Ok(entries.iter().filter_map(|entry| entry.track_id).collect())
}

async fn touch(db: &mut Db, mut playlist: Playlist) -> Result<(), toasty::Error> {
    toasty::update!(playlist { changed_at: now() })
        .exec(db)
        .await
}

/// Creates a playlist, or, given `playlistId`, replaces its songs.
pub(crate) async fn create(cx: &Cx<'_>, params: &Params) -> Result<Payload, Failure> {
    let mut db = cx.lib.db();
    let track_ids = requested_tracks(&cx.lib, params.get_all("songId")).await?;
    let playlist = match params.get("playlistId") {
        Some(id) => {
            let playlist = load(&cx.lib, playlist_id(id)?).await?;
            if let Some(refusal) = read_only(&playlist) {
                return Err(refusal);
            }
            playlist
        }
        None => {
            toasty::create!(Playlist {
                user_id: cx.lib.owner(),
                name: params.require("name")?,
                public: false,
                created_at: now(),
                changed_at: now(),
            })
            .exec(&mut db)
            .await?
        }
    };
    set_tracks(cx, &mut db, playlist.id, &track_ids).await?;
    let id = playlist.id;
    touch(&mut db, playlist).await?;
    let playlist = load(&cx.lib, id).await?;
    Ok(describe(&cx.lib, &playlist, cx.user, true).await?.into())
}

pub(crate) async fn update(cx: &Cx<'_>, params: &Params) -> Result<Payload, Failure> {
    let mut db = cx.lib.db();
    let mut playlist = load(&cx.lib, playlist_id(params.require("playlistId")?)?).await?;
    if playlist.watch_id.is_some() {
        return Err(read_only(&playlist).expect("a mirror is read-only"));
    }
    let name = params.get("name").unwrap_or(&playlist.name).to_owned();
    let comment = params
        .get("comment")
        .map(str::to_owned)
        .or_else(|| playlist.comment.clone());
    let public = match params.get("public") {
        Some(public) => public == "true",
        None => playlist.public,
    };

    let removed: HashSet<usize> = params
        .get_all("songIndexToRemove")
        .filter_map(|index| index.parse().ok())
        .collect();
    let added = requested_tracks(&cx.lib, params.get_all("songIdToAdd")).await?;
    if !removed.is_empty() || !added.is_empty() {
        // A smart playlist may be renamed, but its songs follow its rules.
        if let Some(refusal) = read_only(&playlist) {
            return Err(refusal);
        }
        let mut track_ids: Vec<u64> = current_tracks(&mut db, playlist.id)
            .await?
            .into_iter()
            .enumerate()
            .filter(|(index, _)| !removed.contains(index))
            .map(|(_, track_id)| track_id)
            .collect();
        track_ids.extend(added);
        set_tracks(cx, &mut db, playlist.id, &track_ids).await?;
    }
    toasty::update!(playlist {
        name,
        comment,
        public,
        changed_at: now(),
    })
    .exec(&mut db)
    .await?;
    Ok(Payload::default())
}

pub(crate) async fn delete(cx: &Cx<'_>, params: &Params) -> Result<Payload, Failure> {
    let mut db = cx.lib.db();
    let playlist = load(&cx.lib, playlist_id(params.require("id")?)?).await?;
    if playlist.watch_id.is_some() {
        return Err(read_only(&playlist).expect("a mirror is read-only"));
    }
    // Its tracks are kept; those nothing else claims become orphans.
    set_tracks(cx, &mut db, playlist.id, &[]).await?;
    playlist.delete().exec(&mut db).await?;
    Ok(Payload::default())
}
