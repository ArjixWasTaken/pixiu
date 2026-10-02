//! Enriching albums, with fake MusicBrainz, Cover Art Archive, LRCLIB and
//! Wikipedia.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

use pixiu_db::{
    Album, Artist, Db, Enrichment, Job, JobKind, Lyrics, LyricsSource, Track, now, toasty,
};
use pixiu_enrich::{
    ArtistInfo, BoxFuture, Candidate, Credit, EnrichError, FoundLyrics, LocalAlbum, LyricsQuery,
    Release, ReleaseTrack, Sources,
};
use pixiu_jobs::{
    Executor, Jobs, Outcome,
    enrich::{self, NoPlatformLyrics, PlatformLyrics, Request},
    queue::EnrichJob,
};
use pixiu_treasury::{Claim, Provenance, Treasury, tags};

/// The user every test library and job belongs to.
const OWNER: u64 = 1;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/audio")
        .join(name)
}

#[derive(Default)]
struct FakeSources {
    candidates: Vec<Candidate>,
    releases: HashMap<String, Release>,
    lyrics: HashMap<String, FoundLyrics>,
    searches: Mutex<u32>,
}

impl Sources for FakeSources {
    fn search<'a>(
        &'a self,
        _album: &'a LocalAlbum,
    ) -> BoxFuture<'a, Result<Vec<Candidate>, EnrichError>> {
        *self.searches.lock().unwrap() += 1;
        Box::pin(async move { Ok(self.candidates.clone()) })
    }

    fn release<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Release, EnrichError>> {
        Box::pin(async move { self.releases.get(id).cloned().ok_or(EnrichError::NotFound) })
    }

    fn front_cover<'a>(
        &'a self,
        _release_id: &'a str,
    ) -> BoxFuture<'a, Result<Option<Vec<u8>>, EnrichError>> {
        Box::pin(async move { Ok(Some(std::fs::read(fixture("folder.jpg")).unwrap())) })
    }

    fn lyrics<'a>(
        &'a self,
        query: &'a LyricsQuery,
    ) -> BoxFuture<'a, Result<Option<FoundLyrics>, EnrichError>> {
        Box::pin(async move { Ok(self.lyrics.get(&query.title).cloned()) })
    }

    fn artist_info<'a>(
        &'a self,
        artist_mbid: &'a str,
    ) -> BoxFuture<'a, Result<Option<ArtistInfo>, EnrichError>> {
        Box::pin(async move {
            Ok((artist_mbid == "art-test").then(|| ArtistInfo {
                bio: "Test Artist makes test music.".to_owned(),
                url: "https://en.wikipedia.org/wiki/Test_Artist".to_owned(),
                image_url: Some("https://example.com/test-artist.jpg".to_owned()),
            }))
        })
    }

    fn image<'a>(&'a self, _url: &'a str) -> BoxFuture<'a, Result<Vec<u8>, EnrichError>> {
        Box::pin(async move { Ok(std::fs::read(fixture("folder.jpg")).unwrap()) })
    }
}

struct YouTubeLyrics;

impl PlatformLyrics for YouTubeLyrics {
    fn lyrics<'a>(
        &'a self,
        _key: &'a pixiu_db::SourceKey,
    ) -> pixiu_jobs::warden::BoxFuture<'a, Option<(String, String)>> {
        Box::pin(async { Some(("Sung words".to_owned(), "Source: Somebody".to_owned())) })
    }
}

struct Hoard {
    _dir: tempfile::TempDir,
    db: Db,
    treasury: Treasury,
    album_id: u64,
}

/// "Test Album" by "Test Artist": First Light (1) and Second Wind (2).
async fn hoard() -> Hoard {
    let dir = tempfile::tempdir().unwrap();
    let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    let treasury = Treasury::new(
        db.clone(),
        dir.path().join("treasure"),
        dir.path().join("cache"),
    );
    let staging = dir.path().join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    let mut album_id = 0;
    for name in ["01-first-light.flac", "02-second-wind.mp3"] {
        let staged = staging.join(name);
        std::fs::copy(fixture(name), &staged).unwrap();
        let info = tags::read(&staged).unwrap();
        album_id = treasury
            .ingest(
                OWNER,
                &staged,
                &info,
                None,
                Provenance::offering("upload.flac", None),
                Claim::offering(),
            )
            .await
            .unwrap()
            .album_id;
    }
    Hoard {
        _dir: dir,
        db,
        treasury,
        album_id,
    }
}

fn credit(name: &str, artists: &[(&str, &str)]) -> Credit {
    Credit {
        name: name.to_owned(),
        artists: artists
            .iter()
            .map(|(id, name)| ((*id).to_owned(), (*name).to_owned()))
            .collect(),
    }
}

fn release(id: &str, title: &str, tracks: &[(&str, u64)]) -> Release {
    let artist = credit("Test Artist", &[("art-test", "Test Artist")]);
    Release {
        id: id.to_owned(),
        title: title.to_owned(),
        artist: artist.clone(),
        date: Some("2024-05-01".to_owned()),
        country: None,
        release_group_id: Some(format!("rg-{id}")),
        genre: Some("Electronic".to_owned()),
        has_front_cover: true,
        tracks: tracks
            .iter()
            .enumerate()
            .map(|(index, (title, length))| ReleaseTrack {
                recording_id: format!("rec-{title}"),
                title: (*title).to_owned(),
                artist: if *title == "Second Wind" {
                    credit(
                        "Test Artist feat. Guest",
                        &[("art-test", "Test Artist"), ("art-guest", "Guest")],
                    )
                } else {
                    artist.clone()
                },
                length_ms: Some(*length),
                position: u32::try_from(index + 1).unwrap(),
                disc: 1,
                isrcs: vec![format!("ISRC{}", index + 1)],
            })
            .collect(),
    }
}

fn candidate(id: &str) -> Candidate {
    Candidate {
        id: id.to_owned(),
        title: "Test Album".to_owned(),
        artist: "Test Artist".to_owned(),
        date: None,
        country: None,
        format: None,
        track_count: 2,
        score: 0.9,
    }
}

async fn tracks(db: &mut Db, album_id: u64) -> Vec<Track> {
    let mut tracks = Track::filter_by_album_id(album_id).exec(db).await.unwrap();
    tracks.sort_by_key(|track| track.track_number);
    tracks
}

#[tokio::test]
async fn a_certain_match_rewrites_the_album() {
    let mut hoard = hoard().await;
    let before = tracks(&mut hoard.db, hoard.album_id).await;
    let durations: Vec<u64> = before.iter().map(|track| track.duration_ms).collect();
    let old_cover = Album::get_by_id(&mut hoard.db, &hoard.album_id)
        .await
        .unwrap()
        .cover
        .unwrap();
    let sources = FakeSources {
        candidates: vec![candidate("rel-test")],
        releases: HashMap::from([(
            "rel-test".to_owned(),
            release(
                "rel-test",
                "Test Album: Remastered",
                &[("First Light", durations[0]), ("Second Wind", durations[1])],
            ),
        )]),
        lyrics: HashMap::from([(
            "First Light".to_owned(),
            FoundLyrics {
                synced: Some("[00:00.10]Light".to_owned()),
                plain: Some("Light".to_owned()),
                instrumental: false,
            },
        )]),
        ..FakeSources::default()
    };
    let request = Request {
        album_id: hoard.album_id,
        release: None,
        fresh: false,
        genres_only: false,
    };
    let summary = enrich::enrich(&hoard.treasury, &sources, &NoPlatformLyrics, &request)
        .await
        .unwrap();
    assert!(summary.contains("Matched"), "{summary}");

    let album = Album::get_by_id(&mut hoard.db, &hoard.album_id)
        .await
        .unwrap();
    assert_eq!(album.title, "Test Album: Remastered");
    assert_eq!(album.enrichment, Some(Enrichment::Matched));
    assert_eq!(album.mbid.as_deref(), Some("rel-test"));
    assert_eq!(album.rg_mbid.as_deref(), Some("rg-rel-test"));
    // The files' tags name a genre: MusicBrainz's does not replace it.
    assert_eq!(album.genre.as_deref(), Some("Ambient"));
    // The Cover Art Archive's cover is larger, so it replaced the file's,
    // which nothing shows any more.
    let cover = album.cover.as_deref().unwrap();
    assert!(cover.starts_with(".store/images/") && cover.ends_with(".jpg"));
    assert!(hoard.treasury.resolve(cover).is_file());
    assert!(!hoard.treasury.resolve(&old_cover).exists());

    let tracks = tracks(&mut hoard.db, hoard.album_id).await;
    assert!(
        tracks
            .iter()
            .all(|track| track.genre.as_deref() == Some("Ambient"))
    );
    assert_eq!(tracks[1].mbid.as_deref(), Some("rec-Second Wind"));
    assert_eq!(tracks[1].artist_credit, "Test Artist feat. Guest");
    // Only the database changed: the stored files are as they arrived.
    for (track, old) in tracks.iter().zip(&before) {
        assert_eq!(track.path, old.path);
    }
    let kept = tags::read(&hoard.treasury.resolve(&tracks[0].path)).unwrap();
    assert_eq!(kept.album.as_deref(), Some("Test Album"));
    assert_eq!(kept.mbid, None);

    // The artist learned their id, a biography and a picture.
    let artist = Artist::get_by_id(&mut hoard.db, &album.artist_id)
        .await
        .unwrap();
    assert_eq!(artist.mbid.as_deref(), Some("art-test"));
    assert_eq!(artist.bio.as_deref(), Some("Test Artist makes test music."));
    assert!(artist.image.is_some());

    // Lyrics: LRCLIB had the first; nobody had the second.
    let first = Lyrics::get_by_track_id(&mut hoard.db, &tracks[0].id)
        .await
        .unwrap();
    assert_eq!(first.source, LyricsSource::Lrclib);
    assert_eq!(first.synced.as_deref(), Some("[00:00.10]Light"));
    let second = Lyrics::get_by_track_id(&mut hoard.db, &tracks[1].id)
        .await
        .unwrap();
    assert_eq!(second.source, LyricsSource::Missing);

    // Enriching again sticks to the release: no new search.
    enrich::enrich(&hoard.treasury, &sources, &NoPlatformLyrics, &request)
        .await
        .unwrap();
    assert_eq!(*sources.searches.lock().unwrap(), 1);
}

#[tokio::test]
async fn doubtful_matches_wait_for_the_admin() {
    let mut hoard = hoard().await;
    let before = tracks(&mut hoard.db, hoard.album_id).await;
    let durations: Vec<u64> = before.iter().map(|track| track.duration_ms).collect();
    // Only one of the two tracks is on this release.
    let sources = FakeSources {
        candidates: vec![candidate("rel-partial")],
        releases: HashMap::from([(
            "rel-partial".to_owned(),
            release(
                "rel-partial",
                "Test Album",
                &[("First Light", durations[0]), ("Other", 999_000)],
            ),
        )]),
        ..FakeSources::default()
    };
    let automatic = Request {
        album_id: hoard.album_id,
        release: None,
        fresh: false,
        genres_only: false,
    };
    let summary = enrich::enrich(&hoard.treasury, &sources, &YouTubeLyrics, &automatic)
        .await
        .unwrap();
    assert!(summary.contains("pick one"), "{summary}");
    let album = Album::get_by_id(&mut hoard.db, &hoard.album_id)
        .await
        .unwrap();
    assert_eq!(album.enrichment, Some(Enrichment::Review));
    assert_eq!(album.title, "Test Album", "nothing changed yet");
    let candidates: Vec<Candidate> =
        serde_json::from_str(album.candidates.as_deref().unwrap()).unwrap();
    assert_eq!(candidates[0].id, "rel-partial");
    assert!(candidates[0].score >= 0.5 && candidates[0].score < 0.85);

    // The admin picks it: the track it has is tagged from it.
    let picked = Request {
        album_id: hoard.album_id,
        release: Some("rel-partial".to_owned()),
        fresh: false,
        genres_only: false,
    };
    enrich::enrich(&hoard.treasury, &sources, &YouTubeLyrics, &picked)
        .await
        .unwrap();
    let album = Album::get_by_id(&mut hoard.db, &hoard.album_id)
        .await
        .unwrap();
    assert_eq!(album.enrichment, Some(Enrichment::Matched));
    assert!(album.candidates.is_none());
    let tracks = tracks(&mut hoard.db, hoard.album_id).await;
    assert_eq!(tracks[0].mbid.as_deref(), Some("rec-First Light"));
    assert_eq!(tracks[1].mbid, None);
}

#[tokio::test]
async fn unknown_albums_stay_as_they_are() {
    let mut hoard = hoard().await;
    let sources = FakeSources::default();
    let request = Request {
        album_id: hoard.album_id,
        release: None,
        fresh: false,
        genres_only: false,
    };
    let summary = enrich::enrich(&hoard.treasury, &sources, &YouTubeLyrics, &request)
        .await
        .unwrap();
    assert!(summary.contains("nothing like it"), "{summary}");
    let album = Album::get_by_id(&mut hoard.db, &hoard.album_id)
        .await
        .unwrap();
    assert_eq!(album.enrichment, Some(Enrichment::Unmatched));
    assert_eq!(album.title, "Test Album");
    // Offered files have no YouTube id, so no YouTube lyrics either.
    for track in tracks(&mut hoard.db, hoard.album_id).await {
        let lyrics = Lyrics::get_by_track_id(&mut hoard.db, &track.id)
            .await
            .unwrap();
        assert_eq!(lyrics.source, LyricsSource::Missing);
    }
}

#[tokio::test]
async fn instrumentals_are_known_as_such() {
    let mut hoard = hoard().await;
    // As if downloaded from YouTube Music, which has lyrics for everything.
    for mut track in tracks(&mut hoard.db, hoard.album_id).await {
        let key = format!("youtube_music:video-{}", track.id);
        toasty::update!(track {
            source_key: Some(key)
        })
        .exec(&mut hoard.db)
        .await
        .unwrap();
    }
    let sources = FakeSources {
        lyrics: HashMap::from([(
            "First Light".to_owned(),
            FoundLyrics {
                instrumental: true,
                ..FoundLyrics::default()
            },
        )]),
        ..FakeSources::default()
    };
    let request = Request {
        album_id: hoard.album_id,
        release: None,
        fresh: false,
        genres_only: false,
    };
    let summary = enrich::enrich(&hoard.treasury, &sources, &YouTubeLyrics, &request)
        .await
        .unwrap();
    assert!(summary.ends_with("Lyrics for 1 tracks."), "{summary}");

    // LRCLIB knows the first has no words, so YouTube Music is not asked.
    let tracks = tracks(&mut hoard.db, hoard.album_id).await;
    let first = Lyrics::get_by_track_id(&mut hoard.db, &tracks[0].id)
        .await
        .unwrap();
    assert_eq!(first.source, LyricsSource::Instrumental);
    assert_eq!((first.synced, first.plain), (None, None));
    let second = Lyrics::get_by_track_id(&mut hoard.db, &tracks[1].id)
        .await
        .unwrap();
    assert_eq!(second.source, LyricsSource::YouTubeMusic);
    assert_eq!(second.plain.as_deref(), Some("Sung words"));

    // Later LRCLIB has words after all. That waits for a lookup the admin
    // asks for.
    let sources = FakeSources {
        lyrics: HashMap::from([(
            "First Light".to_owned(),
            FoundLyrics {
                plain: Some("Words after all".to_owned()),
                ..FoundLyrics::default()
            },
        )]),
        ..FakeSources::default()
    };
    let first_source = async |db: &mut Db| {
        Lyrics::get_by_track_id(db, &tracks[0].id)
            .await
            .unwrap()
            .source
    };
    enrich::enrich(&hoard.treasury, &sources, &YouTubeLyrics, &request)
        .await
        .unwrap();
    assert_eq!(
        first_source(&mut hoard.db).await,
        LyricsSource::Instrumental
    );
    let asked = Request {
        fresh: true,
        genres_only: false,
        ..request
    };
    enrich::enrich(&hoard.treasury, &sources, &YouTubeLyrics, &asked)
        .await
        .unwrap();
    assert_eq!(first_source(&mut hoard.db).await, LyricsSource::Lrclib);
}

/// A release credited to two artists, as "Test Artist & Guest".
fn shared_release(durations: &[u64]) -> Release {
    let mut release = release(
        "rel-duo",
        "Test Album",
        &[("First Light", durations[0]), ("Second Wind", durations[1])],
    );
    release.artist = credit(
        "Test Artist & Guest",
        &[("art-test", "Test Artist"), ("art-guest", "Guest")],
    );
    release
}

#[tokio::test]
async fn shared_credits_file_albums_under_the_first_artist() {
    let mut hoard = hoard().await;
    let before = tracks(&mut hoard.db, hoard.album_id).await;
    let durations: Vec<u64> = before.iter().map(|track| track.duration_ms).collect();
    let sources = FakeSources {
        releases: HashMap::from([("rel-duo".to_owned(), shared_release(&durations))]),
        ..FakeSources::default()
    };
    let request = Request {
        album_id: hoard.album_id,
        release: Some("rel-duo".to_owned()),
        fresh: false,
        genres_only: false,
    };
    enrich::enrich(&hoard.treasury, &sources, &NoPlatformLyrics, &request)
        .await
        .unwrap();

    let album = Album::get_by_id(&mut hoard.db, &hoard.album_id)
        .await
        .unwrap();
    let artist = Artist::get_by_id(&mut hoard.db, &album.artist_id)
        .await
        .unwrap();
    assert_eq!(artist.name, "Test Artist");
    assert_eq!(artist.mbid.as_deref(), Some("art-test"));
    let names: Vec<String> = Artist::all()
        .exec(&mut hoard.db)
        .await
        .unwrap()
        .into_iter()
        .map(|artist| artist.name)
        .collect();
    assert!(!names.iter().any(|name| name.contains('&')), "{names:?}");
}

/// Never runs anything: the tests look at what gets queued.
struct Idle;

impl Executor for Idle {
    fn run<'a>(
        &'a self,
        _job: &'a Job,
        _progress: &'a (dyn Fn(u8) + Send + Sync),
    ) -> pixiu_jobs::warden::BoxFuture<'a, Outcome> {
        Box::pin(async { Outcome::Failed("idle".to_owned()) })
    }
}

#[tokio::test]
async fn matched_albums_take_their_genre_once() {
    let mut hoard = hoard().await;
    let before = tracks(&mut hoard.db, hoard.album_id).await;
    let durations: Vec<(&str, u64)> = before
        .iter()
        .map(|track| (track.title.as_str(), track.duration_ms))
        .collect();
    // Matched before genres came from MusicBrainz, from files without a
    // genre; the user gave one track a genre since.
    let mut album = Album::get_by_id(&mut hoard.db, &hoard.album_id)
        .await
        .unwrap();
    let title = album.title.clone();
    toasty::update!(album {
        mbid: Some("rel-test".to_owned()),
        enrichment: Some(Enrichment::Matched),
        genre: Option::<String>::None,
    })
    .exec(&mut hoard.db)
    .await
    .unwrap();
    for (track, genre) in before.iter().zip([Some("Mine".to_owned()), None]) {
        let mut track = Track::get_by_id(&mut hoard.db, &track.id).await.unwrap();
        toasty::update!(track { genre })
            .exec(&mut hoard.db)
            .await
            .unwrap();
    }

    let jobs = Jobs::new(hoard.db.clone(), Box::new(Idle));
    assert_eq!(
        enrich::backfill_genres(&mut hoard.db, &jobs).await.unwrap(),
        1
    );
    assert_eq!(
        enrich::backfill_genres(&mut hoard.db, &jobs).await.unwrap(),
        0
    );
    let job = jobs.unfinished(OWNER).await.unwrap().remove(0);
    let payload: EnrichJob = serde_json::from_str(&job.payload).unwrap();
    assert!(payload.genres_only);
    assert_eq!(payload.release.as_deref(), Some("rel-test"));

    let mut renamed = release("rel-test", "Renamed on MusicBrainz", &durations);
    renamed.genre = Some("Electronic".to_owned());
    let sources = FakeSources {
        releases: HashMap::from([("rel-test".to_owned(), renamed)]),
        ..FakeSources::default()
    };
    let request = Request {
        album_id: payload.album_id,
        release: payload.release,
        fresh: payload.fresh,
        genres_only: payload.genres_only,
    };
    let summary = enrich::enrich(&hoard.treasury, &sources, &NoPlatformLyrics, &request)
        .await
        .unwrap();
    assert_eq!(summary, "Genre: Electronic.");

    // Only the genre changed, and not where the user set one.
    let album = Album::get_by_id(&mut hoard.db, &hoard.album_id)
        .await
        .unwrap();
    assert_eq!(album.genre.as_deref(), Some("Electronic"));
    assert_eq!(album.title, title);
    let after = tracks(&mut hoard.db, hoard.album_id).await;
    assert_eq!(after[0].genre.as_deref(), Some("Mine"));
    assert_eq!(after[1].genre.as_deref(), Some("Electronic"));
    assert_eq!(after[1].title, before[1].title);
    assert_eq!(*sources.searches.lock().unwrap(), 0);
}

#[tokio::test]
async fn albums_under_a_shared_credit_are_repaired_once() {
    let mut hoard = hoard().await;
    let before = tracks(&mut hoard.db, hoard.album_id).await;
    let durations: Vec<u64> = before.iter().map(|track| track.duration_ms).collect();
    // What an earlier píxiū made of a match: the credit as an artist.
    let credit_artist = toasty::create!(Artist {
        user_id: OWNER,
        name: "Test Artist & Guest",
        name_key: "test artist & guest",
        created_at: now(),
    })
    .exec(&mut hoard.db)
    .await
    .unwrap();
    let mut album = Album::get_by_id(&mut hoard.db, &hoard.album_id)
        .await
        .unwrap();
    toasty::update!(album {
        artist_id: credit_artist.id,
        mbid: Some("rel-duo".to_owned()),
        enrichment: Some(Enrichment::Matched),
    })
    .exec(&mut hoard.db)
    .await
    .unwrap();

    let jobs = Jobs::new(hoard.db.clone(), Box::new(Idle));
    let queued = enrich::repair_album_artists(&mut hoard.db, &jobs)
        .await
        .unwrap();
    assert_eq!(queued, 1);
    let job = jobs.unfinished(OWNER).await.unwrap().remove(0);
    assert_eq!(job.kind, JobKind::Enrich);
    let payload: EnrichJob = serde_json::from_str(&job.payload).unwrap();
    assert_eq!(payload.release.as_deref(), Some("rel-duo"));
    // It runs once per hoard.
    assert_eq!(
        enrich::repair_album_artists(&mut hoard.db, &jobs)
            .await
            .unwrap(),
        0
    );

    // Running that lookup moves the album to its artist; the credit's
    // artist goes.
    let sources = FakeSources {
        releases: HashMap::from([("rel-duo".to_owned(), shared_release(&durations))]),
        ..FakeSources::default()
    };
    let request = Request {
        album_id: payload.album_id,
        release: payload.release,
        fresh: payload.fresh,
        genres_only: false,
    };
    enrich::enrich(&hoard.treasury, &sources, &NoPlatformLyrics, &request)
        .await
        .unwrap();
    let album = Album::get_by_id(&mut hoard.db, &hoard.album_id)
        .await
        .unwrap();
    let artist = Artist::get_by_id(&mut hoard.db, &album.artist_id)
        .await
        .unwrap();
    assert_eq!(artist.name, "Test Artist");
    assert!(
        Artist::filter_by_id(credit_artist.id)
            .first()
            .exec(&mut hoard.db)
            .await
            .unwrap()
            .is_none()
    );
    // The file stays in the store, as it was.
    let path = &tracks(&mut hoard.db, hoard.album_id).await[0].path;
    assert!(path.starts_with(".store/audio/"), "{path}");
}

#[tokio::test]
async fn genres_named_the_old_way_are_renamed_once() {
    let mut hoard = hoard().await;
    let before = tracks(&mut hoard.db, hoard.album_id).await;
    let mut album = Album::get_by_id(&mut hoard.db, &hoard.album_id)
        .await
        .unwrap();
    toasty::update!(album {
        genre: Some("Drum And Bass".to_owned())
    })
    .exec(&mut hoard.db)
    .await
    .unwrap();
    // One genre the old naming made, one the files named.
    for (track, genre) in before.iter().zip(["J-pop", "UKG"]) {
        let mut track = Track::get_by_id(&mut hoard.db, &track.id).await.unwrap();
        toasty::update!(track {
            genre: Some(genre.to_owned())
        })
        .exec(&mut hoard.db)
        .await
        .unwrap();
    }

    assert_eq!(enrich::repair_genre_names(&mut hoard.db).await.unwrap(), 2);
    let album = Album::get_by_id(&mut hoard.db, &hoard.album_id)
        .await
        .unwrap();
    assert_eq!(album.genre.as_deref(), Some("Drum and Bass"));
    let after = tracks(&mut hoard.db, hoard.album_id).await;
    assert_eq!(after[0].genre.as_deref(), Some("J-Pop"));
    assert_eq!(after[1].genre.as_deref(), Some("UKG"));
    // It runs once per hoard.
    assert_eq!(enrich::repair_genre_names(&mut hoard.db).await.unwrap(), 0);
}
