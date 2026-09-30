//! Enriching albums: MusicBrainz metadata, Cover Art Archive covers,
//! lyrics, and Wikipedia biographies.
//!
//! Only a certain match rewrites an album; a plausible one waits for the
//! admin to pick, and none at all leaves the album as it is. Lyrics are
//! looked for either way.

use std::time::Duration;

use jiff::SignedDuration;
use pixiu_db::{Album, Artist, Db, Enrichment, Lyrics, LyricsSource, Setting, Track, now, toasty};
use pixiu_enrich::{
    Candidate, EnrichError, LocalAlbum, LocalTrack, LyricsQuery, Pairing, Release, Sources,
    looks_synced,
    matching::{PLAUSIBLE, is_certain, pair},
};
use pixiu_treasury::{AlbumEdit, ArtistRef, Cover, TrackEdit, Treasury, covers, tags};

use crate::{
    queue::{Jobs, NewJob},
    warden::BoxFuture,
};

/// How many search results are looked at closely.
const LOOK_CLOSER: usize = 3;

/// How long "nothing on Wikipedia" or "no lyrics" stands before asking
/// again.
const RECHECK_AFTER: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Lyrics a streaming platform has for its tracks.
pub trait PlatformLyrics: Send + Sync {
    /// Plain lyrics of a YouTube video, with their credit line.
    fn lyrics<'a>(&'a self, video_id: &'a str) -> BoxFuture<'a, Option<(String, String)>>;
}

/// No platform lyrics: for albums nobody downloaded, and for tests.
pub struct NoPlatformLyrics;

impl PlatformLyrics for NoPlatformLyrics {
    fn lyrics<'a>(&'a self, _video_id: &'a str) -> BoxFuture<'a, Option<(String, String)>> {
        Box::pin(async { None })
    }
}

/// What to enrich.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub album_id: u64,
    /// A release the admin picked.
    pub release: Option<String>,
    /// Search again even if the album is matched.
    pub fresh: bool,
}

fn failed(error: impl ToString) -> String {
    error.to_string()
}

fn stale(checked: Option<jiff::Timestamp>) -> bool {
    let limit =
        SignedDuration::from_secs(i64::try_from(RECHECK_AFTER.as_secs()).unwrap_or(i64::MAX));
    checked.is_none_or(|checked| now().duration_since(checked) > limit)
}

/// Enriches an album; see the module docs. Returns a summary.
///
/// # Errors
///
/// Fails when MusicBrainz cannot be asked, or on database and file errors.
pub async fn enrich(
    treasury: &Treasury,
    sources: &dyn Sources,
    platform: &dyn PlatformLyrics,
    request: &Request,
) -> Result<String, String> {
    let mut db = treasury.db();
    let Some(album) = Album::filter_by_id(request.album_id)
        .first()
        .exec(&mut db)
        .await
        .map_err(failed)?
    else {
        return Ok("The album is gone.".to_owned());
    };

    let mut summary = identify(treasury, sources, &mut db, album, request).await?;
    // A lookup the admin asked for may have fixed the titles: look again.
    let asked = request.fresh || request.release.is_some();
    let found = find_lyrics(
        treasury,
        sources,
        platform,
        &mut db,
        request.album_id,
        asked,
    )
    .await?;
    if found > 0 {
        summary.push_str(&format!(" Lyrics for {found} tracks."));
    }
    Ok(summary)
}

/// The album as the matching rules see it.
async fn local_album(db: &mut Db, album: &Album) -> Result<LocalAlbum, toasty::Error> {
    let artist = Artist::get_by_id(&mut *db, &album.artist_id).await?;
    let tracks = Track::filter_by_album_id(album.id).exec(&mut *db).await?;
    Ok(LocalAlbum {
        title: album.title.clone(),
        artist: artist.name,
        year: album.year,
        mbid: album.mbid.clone(),
        tracks: tracks
            .into_iter()
            .map(|track| LocalTrack {
                id: track.id,
                title: track.title,
                artist: track.artist_credit,
                duration_ms: track.duration_ms,
                track_number: track.track_number,
                disc_number: track.disc_number,
                isrc: track.isrc,
            })
            .collect(),
    })
}

async fn record(
    db: &mut Db,
    album_id: u64,
    state: Enrichment,
    candidates: Option<String>,
) -> Result<(), toasty::Error> {
    let mut album = Album::get_by_id(&mut *db, &album_id).await?;
    toasty::update!(album {
        enrichment: Some(state),
        candidates,
        enriched_at: Some(now()),
    })
    .exec(db)
    .await
}

/// Finds the album on MusicBrainz and applies what is certain.
async fn identify(
    treasury: &Treasury,
    sources: &dyn Sources,
    db: &mut Db,
    album: Album,
    request: &Request,
) -> Result<String, String> {
    let local = local_album(db, &album).await.map_err(failed)?;
    if local.tracks.is_empty() {
        return Ok("The album has no tracks.".to_owned());
    }

    // A release the admin picked, or the one the album is matched to.
    let chosen = request
        .release
        .clone()
        .or_else(|| (!request.fresh).then(|| album.mbid.clone()).flatten());
    if let Some(id) = chosen {
        let release = match sources.release(&id).await {
            Ok(release) => release,
            Err(EnrichError::NotFound) => return Err(format!("MusicBrainz has no release {id}")),
            Err(error) => return Err(failed(error)),
        };
        let pairing = pair(&local, &release);
        // The admin's word is enough; a stored match is kept for new tracks.
        apply(treasury, sources, db, album.id, &release, &pairing).await?;
        return Ok(format!("Tagged from “{}”.", release.title));
    }

    let candidates = sources.search(&local).await.map_err(failed)?;
    let mut best: Option<(Pairing, Release)> = None;
    let mut reviewed = Vec::new();
    for candidate in candidates.iter().take(LOOK_CLOSER) {
        let release = sources.release(&candidate.id).await.map_err(failed)?;
        let pairing = pair(&local, &release);
        reviewed.push(Candidate {
            score: pairing.confidence,
            ..candidate.clone()
        });
        if best
            .as_ref()
            .is_none_or(|(current, _)| pairing.confidence > current.confidence)
        {
            best = Some((pairing, release));
        }
    }
    // Only releases that were looked at closely, and share tracks with the
    // album, are worth the admin's time.
    reviewed.retain(|candidate| candidate.score >= PLAUSIBLE);
    reviewed.sort_by(|a, b| b.score.total_cmp(&a.score));

    match best {
        Some((pairing, release)) if is_certain(&pairing) => {
            apply(treasury, sources, db, album.id, &release, &pairing).await?;
            Ok(format!("Matched to “{}”.", release.title))
        }
        _ if !reviewed.is_empty() => {
            let json = serde_json::to_string(&reviewed).map_err(failed)?;
            record(db, album.id, Enrichment::Review, Some(json))
                .await
                .map_err(failed)?;
            Ok(format!(
                "{} releases might be it; pick one.",
                reviewed.len()
            ))
        }
        _ => {
            record(db, album.id, Enrichment::Unmatched, None)
                .await
                .map_err(failed)?;
            Ok("MusicBrainz knows nothing like it.".to_owned())
        }
    }
}

/// The artist a credit stands for: the first credited. A credit shared by
/// several ("A & B") is not an artist of its own: made one, it would split
/// A's albums across as many artists as A has collaborations. The full
/// credit stays on the tracks.
fn primary(credit: &pixiu_enrich::Credit) -> ArtistRef {
    credit.artists.first().map_or_else(
        || ArtistRef {
            name: credit.name.clone(),
            mbid: None,
        },
        |(id, name)| ArtistRef {
            name: name.clone(),
            mbid: Some(id.clone()),
        },
    )
}

/// Retags and refiles the album from the release, then fetches the
/// release's cover and the artist's biography.
async fn apply(
    treasury: &Treasury,
    sources: &dyn Sources,
    db: &mut Db,
    album_id: u64,
    release: &Release,
    pairing: &Pairing,
) -> Result<(), String> {
    let album = Album::get_by_id(&mut *db, &album_id)
        .await
        .map_err(failed)?;
    let discs = release
        .tracks
        .iter()
        .map(|track| track.disc)
        .max()
        .unwrap_or(1);
    let mut tracks = Vec::new();
    for &(track_id, index) in &pairing.tracks {
        let remote = &release.tracks[index];
        let local = Track::get_by_id(&mut *db, &track_id)
            .await
            .map_err(failed)?;
        tracks.push(TrackEdit {
            track_id,
            title: remote.title.clone(),
            artist_credit: remote.artist.name.clone(),
            artist: primary(&remote.artist),
            track_number: Some(remote.position),
            // Disc numbers only for albums that have several.
            disc_number: if discs > 1 {
                Some(remote.disc)
            } else {
                local.disc_number
            },
            mbid: Some(remote.recording_id.clone()),
            isrc: remote.isrcs.first().cloned(),
        });
    }
    treasury
        .edit_album(&AlbumEdit {
            album_id,
            title: release.title.clone(),
            artist: primary(&release.artist),
            year: release.year().or(album.year),
            mbid: Some(release.id.clone()),
            rg_mbid: release.release_group_id.clone(),
            tracks,
        })
        .await
        .map_err(failed)?;
    record(db, album_id, Enrichment::Matched, None)
        .await
        .map_err(failed)?;

    if release.has_front_cover {
        better_cover(treasury, sources, db, album_id, &release.id).await;
    }
    let album = Album::get_by_id(&mut *db, &album_id)
        .await
        .map_err(failed)?;
    artist_info(treasury, sources, db, album.artist_id).await;
    Ok(())
}

/// Takes the Cover Art Archive's front cover when it is larger than the
/// album's. Failures are logged: covers are a nicety.
async fn better_cover(
    treasury: &Treasury,
    sources: &dyn Sources,
    db: &mut Db,
    album_id: u64,
    release_id: &str,
) {
    let data = match sources.front_cover(release_id).await {
        Ok(Some(data)) => data,
        Ok(None) => return,
        Err(error) => {
            tracing::warn!(%error, release_id, "cannot fetch the cover");
            return;
        }
    };
    let (Some(mime), Some((width, _))) = (covers::mime_of(&data), covers::dimensions(&data)) else {
        return;
    };
    let current_width = match Album::get_by_id(&mut *db, &album_id).await {
        Ok(album) => match album.cover {
            Some(cover) => tokio::fs::read(treasury.resolve(&cover))
                .await
                .ok()
                .and_then(|data| covers::dimensions(&data))
                .map_or(0, |(width, _)| width),
            None => 0,
        },
        Err(_) => return,
    };
    if width > current_width {
        let cover = Cover {
            data,
            mime: mime.to_owned(),
        };
        if let Err(error) = treasury.replace_cover(album_id, &cover).await {
            tracing::warn!(%error, album_id, "cannot replace the cover");
        }
    }
}

/// Fetches an artist's biography and picture, unless fetched lately.
async fn artist_info(treasury: &Treasury, sources: &dyn Sources, db: &mut Db, artist_id: u64) {
    let Ok(artist) = Artist::get_by_id(&mut *db, &artist_id).await else {
        return;
    };
    let Some(mbid) = artist.mbid.clone() else {
        return;
    };
    if !stale(artist.info_fetched_at) {
        return;
    }
    let info = match sources.artist_info(&mbid).await {
        Ok(info) => info,
        Err(error) => {
            tracing::warn!(%error, artist = %artist.name, "cannot fetch the artist's biography");
            return;
        }
    };
    let image_url = info.as_ref().and_then(|info| info.image_url.clone());
    let bio = info.map(|info| (info.bio, info.url));
    if let Err(error) = treasury.set_artist_bio(artist_id, bio).await {
        tracing::warn!(%error, "cannot record a biography");
    }
    if let Some(url) = image_url {
        match sources.image(&url).await {
            Ok(data) => {
                if let Err(error) = treasury.set_artist_image(artist_id, &data).await {
                    tracing::warn!(%error, "cannot store an artist picture");
                }
            }
            Err(error) => tracing::warn!(%error, "cannot fetch an artist picture"),
        }
    }
}

/// Looks for lyrics of the album's tracks that have none: in the file, on
/// LRCLIB, then on YouTube Music. Returns how many were found. Unless
/// `again`, tracks LRCLIB knows as instrumental are not looked at again, and
/// those with nothing found only after a while.
async fn find_lyrics(
    treasury: &Treasury,
    sources: &dyn Sources,
    platform: &dyn PlatformLyrics,
    db: &mut Db,
    album_id: u64,
    again: bool,
) -> Result<usize, String> {
    let album = Album::get_by_id(&mut *db, &album_id)
        .await
        .map_err(failed)?;
    let tracks = Track::filter_by_album_id(album_id)
        .exec(&mut *db)
        .await
        .map_err(failed)?;
    let mut found = 0;
    for track in tracks {
        let existing = Lyrics::filter_by_track_id(track.id)
            .first()
            .exec(&mut *db)
            .await
            .map_err(failed)?;
        let recheck = match &existing {
            None => true,
            Some(lyrics) if lyrics.source.has_words() => false,
            Some(lyrics) => {
                again || (lyrics.source == LyricsSource::Missing && stale(Some(lyrics.fetched_at)))
            }
        };
        if !recheck {
            continue;
        }

        let (source, synced, plain) = lyrics_for(treasury, sources, platform, &album, &track).await;
        if source.has_words() {
            found += 1;
        }
        if let Some(existing) = existing {
            existing.delete().exec(&mut *db).await.map_err(failed)?;
        }
        toasty::create!(Lyrics {
            track_id: track.id,
            source,
            synced,
            plain,
            fetched_at: now(),
        })
        .exec(&mut *db)
        .await
        .map_err(failed)?;
    }
    Ok(found)
}

async fn lyrics_for(
    treasury: &Treasury,
    sources: &dyn Sources,
    platform: &dyn PlatformLyrics,
    album: &Album,
    track: &Track,
) -> (LyricsSource, Option<String>, Option<String>) {
    let file = treasury.resolve(&track.path);
    if let Ok(Some(text)) = tokio::task::spawn_blocking(move || tags::lyrics(&file)).await {
        return if looks_synced(&text) {
            (LyricsSource::File, Some(text), None)
        } else {
            (LyricsSource::File, None, Some(text))
        };
    }

    let query = LyricsQuery {
        title: track.title.clone(),
        artist: track.artist_credit.clone(),
        album: album.title.clone(),
        duration_secs: (track.duration_ms + 500) / 1000,
    };
    match sources.lyrics(&query).await {
        Ok(Some(lyrics)) if lyrics.instrumental => return (LyricsSource::Instrumental, None, None),
        Ok(Some(lyrics)) => return (LyricsSource::Lrclib, lyrics.synced, lyrics.plain),
        Ok(None) => {}
        Err(error) => tracing::warn!(%error, track = track.id, "cannot ask LRCLIB for lyrics"),
    }

    if let Some(video_id) = &track.ytm_video_id
        && let Some((body, _credit)) = platform.lyrics(video_id).await
    {
        return (LyricsSource::YouTubeMusic, None, Some(body));
    }
    (LyricsSource::Missing, None, None)
}

/// The setting that records [`repair_album_artists`] ran.
const REPAIRED_ALBUM_ARTISTS: &str = "repair.album-artists";

/// Once per server: looks up again, from the release they were matched to,
/// the albums an earlier píxiū filed under a shared credit ("A & B") as if
/// it were an artist. Those album artists are the ones MusicBrainz-matched
/// albums have without a MusicBrainz id. The lookups move each album to its
/// first credited artist; the credit's artist
/// goes once it has no albums left. Returns how many albums were queued.
///
/// # Errors
///
/// Fails on database errors.
pub async fn repair_album_artists(db: &mut Db, jobs: &Jobs) -> Result<usize, toasty::Error> {
    if Setting::filter_by_key(REPAIRED_ALBUM_ARTISTS)
        .first()
        .exec(&mut *db)
        .await?
        .is_some()
    {
        return Ok(0);
    }
    let unnamed: std::collections::HashSet<u64> = Artist::all()
        .exec(&mut *db)
        .await?
        .into_iter()
        .filter(|artist| artist.mbid.is_none())
        .map(|artist| artist.id)
        .collect();
    let mut queued = 0;
    for album in Album::all().exec(&mut *db).await? {
        let Some(release) = album.mbid.clone() else {
            continue;
        };
        if album.enrichment == Some(Enrichment::Matched) && unnamed.contains(&album.artist_id) {
            jobs.enqueue(
                album.user_id,
                NewJob::enrich(
                    album.id,
                    &format!("Look up {}", album.title),
                    Some(release),
                    false,
                ),
            )
            .await?;
            queued += 1;
        }
    }
    toasty::create!(Setting {
        key: REPAIRED_ALBUM_ARTISTS,
        value: now().to_string(),
    })
    .exec(&mut *db)
    .await?;
    if queued > 0 {
        tracing::info!(
            albums = queued,
            "re-filing albums credited to several artists"
        );
    }
    Ok(queued)
}
