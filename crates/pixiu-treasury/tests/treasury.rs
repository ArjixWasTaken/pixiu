use std::{
    io::Write,
    path::{Path, PathBuf},
};

use pixiu_db::{Album, Artist, ClaimKind, Db, OfferingStatus, Track, TrackClaim};
use pixiu_treasury::{Claim, IngestError, OfferingError, Offerings, Provenance, Treasury, tags};
use tokio::io::AsyncWriteExt;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/audio")
        .join(name)
}

struct Hoard {
    dir: tempfile::TempDir,
    db: Db,
    treasury: Treasury,
}

impl Hoard {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
        let treasury = Treasury::new(
            db.clone(),
            dir.path().join("treasure"),
            dir.path().join("cache"),
        );
        Self { dir, db, treasury }
    }

    /// Copies a fixture into a staging area, like a finished download.
    fn stage(&self, name: &str) -> PathBuf {
        let staging = self.dir.path().join("staging");
        std::fs::create_dir_all(&staging).unwrap();
        let path = staging.join(name);
        std::fs::copy(fixture(name), &path).unwrap();
        path
    }

    fn offerings(&self) -> Offerings {
        Offerings::new(self.dir.path().join("offerings"), self.treasury.clone())
    }
}

#[test]
fn reads_tags_and_properties() {
    let flac = tags::read(&fixture("01-first-light.flac")).unwrap();
    assert_eq!(flac.title.as_deref(), Some("First Light"));
    assert_eq!(flac.artist.as_deref(), Some("Test Artist"));
    assert_eq!(flac.album.as_deref(), Some("Test Album"));
    assert_eq!(flac.album_artist.as_deref(), Some("Test Artist"));
    assert_eq!(flac.track_number, Some(1));
    assert_eq!(flac.disc_number, Some(1));
    assert_eq!(flac.year, Some(2024));
    assert_eq!(flac.genre.as_deref(), Some("Ambient"));
    assert_eq!(
        (flac.suffix.as_str(), flac.content_type.as_str()),
        ("flac", "audio/flac")
    );
    assert_eq!(flac.sample_rate, Some(8000));
    assert_eq!(flac.channels, Some(1));
    assert!(
        (900..=1100).contains(&flac.duration_ms),
        "{}",
        flac.duration_ms
    );
    let cover = flac.cover.expect("embedded cover");
    assert_eq!(cover.mime, "image/png");

    let mp3 = tags::read(&fixture("02-second-wind.mp3")).unwrap();
    assert_eq!(mp3.artist.as_deref(), Some("Test Artist feat. Guest"));
    assert_eq!(mp3.track_number, Some(2));
    assert_eq!(
        (mp3.suffix.as_str(), mp3.content_type.as_str()),
        ("mp3", "audio/mpeg")
    );
    assert!(mp3.cover.is_none());

    let opus = tags::read(&fixture("untagged.opus")).unwrap();
    assert_eq!(opus.title, None);
    assert_eq!(opus.suffix, "opus");

    assert!(tags::read(&fixture("folder.jpg")).is_err());
}

#[tokio::test]
async fn ingest_files_tracks_under_the_layout() {
    let hoard = Hoard::new().await;
    let mut db = hoard.db.clone();

    for name in ["01-first-light.flac", "02-second-wind.mp3"] {
        let staged = hoard.stage(name);
        let info = tags::read(&staged).unwrap();
        hoard
            .treasury
            .ingest(
                &staged,
                &info,
                None,
                Provenance::offering(),
                Claim::offering(),
            )
            .await
            .unwrap();
        assert!(!staged.exists(), "the file was moved, not copied");
    }

    // One artist: the featured guest is not the primary artist.
    let artists = Artist::all().exec(&mut db).await.unwrap();
    assert_eq!(artists.len(), 1);
    assert_eq!(artists[0].name, "Test Artist");

    let albums = Album::all().exec(&mut db).await.unwrap();
    assert_eq!(albums.len(), 1);
    let album = &albums[0];
    assert_eq!(
        (album.title.as_str(), album.year),
        ("Test Album", Some(2024))
    );
    let cover = album.cover.as_deref().expect("cover taken from the FLAC");
    assert_eq!(cover, "Test Artist/2024 - Test Album/cover.png");
    assert!(hoard.treasury.resolve(cover).is_file());

    let mut tracks = Track::all().exec(&mut db).await.unwrap();
    tracks.sort_by_key(|track| track.track_number);
    let paths: Vec<_> = tracks.iter().map(|track| track.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "Test Artist/2024 - Test Album/01-01 First Light.flac",
            "Test Artist/2024 - Test Album/01-02 Second Wind.mp3",
        ]
    );
    assert!(
        tracks
            .iter()
            .all(|track| hoard.treasury.resolve(&track.path).is_file())
    );
    assert_eq!(tracks[1].artist_credit, "Test Artist feat. Guest");
    assert_eq!(tracks[1].artist_id, artists[0].id);

    let claims = TrackClaim::all().exec(&mut db).await.unwrap();
    assert_eq!(claims.len(), 2);
    assert!(claims.iter().all(|claim| claim.kind == ClaimKind::Offering));

    // The same track again is refused, and the new copy stays where it was.
    let staged = hoard.stage("01-first-light.flac");
    let info = tags::read(&staged).unwrap();
    let error = hoard
        .treasury
        .ingest(
            &staged,
            &info,
            None,
            Provenance::offering(),
            Claim::offering(),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, IngestError::Duplicate { track_id } if track_id == tracks[0].id));
    assert!(staged.exists());
}

#[tokio::test]
async fn a_new_layout_moves_the_hoard() {
    let hoard = Hoard::new().await;
    let mut db = hoard.db.clone();
    let offer = |name: &'static str| {
        let staged = hoard.stage(name);
        let treasury = hoard.treasury.clone();
        async move {
            let info = tags::read(&staged).unwrap();
            treasury
                .ingest(
                    &staged,
                    &info,
                    None,
                    Provenance::offering(),
                    Claim::offering(),
                )
                .await
                .unwrap()
        }
    };
    offer("01-first-light.flac").await;

    let template =
        pixiu_treasury::Template::parse("{genre}/{album} ({year})/{track:03}. {title}").unwrap();
    assert_eq!(hoard.treasury.misplaced().await.unwrap(), 0);
    hoard.treasury.set_layout(template.clone()).await.unwrap();
    assert_eq!(hoard.treasury.misplaced().await.unwrap(), 1);
    // New tracks follow the new layout; the old one stays until refiled.
    offer("02-second-wind.mp3").await;
    let paths = |tracks: Vec<Track>| {
        let mut paths: Vec<String> = tracks.into_iter().map(|track| track.path).collect();
        paths.sort();
        paths
    };
    assert_eq!(
        paths(Track::all().exec(&mut db).await.unwrap()),
        [
            "Ambient/Test Album (2024)/002. Second Wind.mp3",
            "Test Artist/2024 - Test Album/01-01 First Light.flac",
        ]
    );

    let seen = std::sync::Mutex::new(Vec::new());
    let albums = hoard
        .treasury
        .refile_all(|done, of| seen.lock().unwrap().push((done, of)))
        .await
        .unwrap();
    assert_eq!(albums, 1);
    assert_eq!(*seen.lock().unwrap(), [(1, 1)]);
    assert_eq!(hoard.treasury.misplaced().await.unwrap(), 0);
    let tracks = Track::all().exec(&mut db).await.unwrap();
    assert!(
        tracks
            .iter()
            .all(|track| hoard.treasury.resolve(&track.path).is_file())
    );
    assert_eq!(
        paths(tracks),
        [
            "Ambient/Test Album (2024)/001. First Light.flac",
            "Ambient/Test Album (2024)/002. Second Wind.mp3",
        ]
    );
    let album = Album::all().exec(&mut db).await.unwrap().remove(0);
    assert_eq!(
        album.cover.as_deref(),
        Some("Ambient/Test Album (2024)/cover.png")
    );
    // The old directories went away with their files.
    assert!(!hoard.treasury.resolve("Test Artist").exists());

    // The setting outlives the process.
    let reopened = Treasury::new(
        hoard.db.clone(),
        hoard.treasury.root(),
        hoard.treasury.cache_dir(),
    );
    assert_ne!(reopened.layout(), template);
    reopened.load_layout().await.unwrap();
    assert_eq!(reopened.layout(), template);
}

fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut buffer);
    for (name, data) in files {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap();
    buffer.into_inner()
}

async fn upload(offerings: &Offerings, batch: &str, name: &str, data: &[u8]) {
    let (_, mut file) = offerings.create_upload(batch, name).await.unwrap();
    file.write_all(data).await.unwrap();
    file.flush().await.unwrap();
}

#[tokio::test]
async fn offerings_are_reviewed_then_absorbed() {
    let hoard = Hoard::new().await;
    let offerings = hoard.offerings();
    let mut db = hoard.db.clone();

    // An album as a zip with a folder image and junk, plus a loose track.
    let batch = Offerings::new_batch();
    let archive = zip_of(&[
        (
            "Test Album/01-first-light.flac",
            &std::fs::read(fixture("01-first-light.flac")).unwrap(),
        ),
        (
            "Test Album/02-second-wind.mp3",
            &std::fs::read(fixture("02-second-wind.mp3")).unwrap(),
        ),
        (
            "Test Album/folder.jpg",
            &std::fs::read(fixture("folder.jpg")).unwrap(),
        ),
        ("Test Album/rip.log", b"EAC log"),
        ("../escape.mp3", b"nope"),
    ]);
    upload(&offerings, &batch, "album.zip", &archive).await;
    upload(
        &offerings,
        &batch,
        "untagged.opus",
        &std::fs::read(fixture("untagged.opus")).unwrap(),
    )
    .await;
    upload(&offerings, &batch, "not-audio.mp3", b"plain text").await;

    let registered = offerings.process_batch(&batch).await.unwrap();
    assert_eq!(registered.len(), 4, "{registered:#?}");
    let unreadable: Vec<_> = registered
        .iter()
        .filter(|offering| offering.status == OfferingStatus::Unreadable)
        .collect();
    assert_eq!(unreadable.len(), 1);
    assert_eq!(unreadable[0].file_name, "not-audio.mp3");
    assert!(
        !hoard.dir.path().join("escape.mp3").exists(),
        "archive entries cannot escape the batch"
    );

    let untagged = registered
        .iter()
        .find(|offering| offering.file_name == "untagged.opus")
        .unwrap();
    assert_eq!(untagged.title, "untagged");
    assert_eq!(untagged.artist, "Unknown Artist");

    // Accepting an unreadable offering fails; discarding it works.
    assert!(matches!(
        offerings.accept(unreadable[0].id).await,
        Err(OfferingError::Unreadable(_))
    ));
    offerings.discard(unreadable[0].id).await.unwrap();

    let failures = offerings.accept_batch(&batch).await.unwrap().failures;
    assert!(failures.is_empty(), "{failures:?}");
    assert!(offerings.pending().await.unwrap().is_empty());
    assert!(
        !hoard.dir.path().join("offerings").join(&batch).exists(),
        "the batch directory is cleaned up"
    );

    let mut paths: Vec<_> = Track::all()
        .exec(&mut db)
        .await
        .unwrap()
        .into_iter()
        .map(|track| track.path)
        .collect();
    paths.sort();
    assert_eq!(
        paths,
        [
            "Test Artist/2024 - Test Album/01-01 First Light.flac",
            "Test Artist/2024 - Test Album/01-02 Second Wind.mp3",
            "Unknown Artist/Unknown Album/untagged.opus",
        ]
    );
    // The embedded FLAC cover wins; the untagged track's album gets the
    // batch's folder image.
    let mut covers: Vec<_> = Album::all()
        .exec(&mut db)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|album| album.cover)
        .collect();
    covers.sort();
    assert_eq!(
        covers,
        [
            "Test Artist/2024 - Test Album/cover.png",
            "Unknown Artist/Unknown Album/cover.jpg",
        ]
    );
}

#[tokio::test]
async fn discarding_a_batch_removes_everything() {
    let hoard = Hoard::new().await;
    let offerings = hoard.offerings();

    let batch = Offerings::new_batch();
    upload(
        &offerings,
        &batch,
        "01-first-light.flac",
        &std::fs::read(fixture("01-first-light.flac")).unwrap(),
    )
    .await;
    assert_eq!(offerings.process_batch(&batch).await.unwrap().len(), 1);

    offerings.discard_batch(&batch).await.unwrap();
    assert!(offerings.pending().await.unwrap().is_empty());
    assert!(!hoard.dir.path().join("offerings").join(&batch).exists());
    assert!(offerings.discard_batch("../etc").await.is_err());
}

#[tokio::test]
async fn downloads_dedupe_by_platform_ids() {
    let hoard = Hoard::new().await;
    let youtube = |video: &str| Provenance::youtube_music(video, Some("MPREb_album".to_owned()));

    let staged = hoard.stage("02-second-wind.mp3");
    let mut info = tags::read(&staged).unwrap();
    info.artists = vec!["Test Artist".to_owned(), "Guest".to_owned()];
    let first = hoard
        .treasury
        .ingest(&staged, &info, None, youtube("video-1"), Claim::offering())
        .await
        .unwrap();
    assert_eq!(first.ytm_video_id.as_deref(), Some("video-1"));

    // The same video again is a duplicate, whatever its tags say.
    let staged = hoard.stage("02-second-wind.mp3");
    let mut info = tags::read(&staged).unwrap();
    info.title = Some("Renamed".to_owned());
    let error = hoard
        .treasury
        .ingest(&staged, &info, None, youtube("video-1"), Claim::offering())
        .await
        .unwrap_err();
    assert!(matches!(error, IngestError::Duplicate { track_id } if track_id == first.id));

    // Another track of the same platform album joins it, even when its
    // album name differs.
    let staged = hoard.stage("01-first-light.flac");
    let mut info = tags::read(&staged).unwrap();
    info.album = Some("Test Album (Deluxe)".to_owned());
    let second = hoard
        .treasury
        .ingest(&staged, &info, None, youtube("video-2"), Claim::offering())
        .await
        .unwrap();
    assert_eq!(second.album_id, first.album_id);

    let mut db = hoard.db.clone();
    let album = Album::get_by_id(&mut db, &first.album_id).await.unwrap();
    assert_eq!(album.ytm_browse_id.as_deref(), Some("MPREb_album"));
}

#[tokio::test]
async fn orphans_are_kept_until_the_admin_deletes_them() {
    let hoard = Hoard::new().await;
    let mut db = hoard.db.clone();
    let mut ids = Vec::new();
    for name in ["01-first-light.flac", "02-second-wind.mp3"] {
        let staged = hoard.stage(name);
        let info = tags::read(&staged).unwrap();
        let track = hoard
            .treasury
            .ingest(
                &staged,
                &info,
                None,
                Provenance::offering(),
                Claim::offering(),
            )
            .await
            .unwrap();
        ids.push(track.id);
    }
    let (first, second) = (ids[0], ids[1]);
    let first_path = hoard
        .treasury
        .resolve(&Track::get_by_id(&mut db, &first).await.unwrap().path);

    // Claims are not doubled.
    let watch = Claim {
        kind: ClaimKind::WatchPlaylist,
        reference: Some("7".into()),
    };
    hoard.treasury.claim(first, &watch).await.unwrap();
    hoard.treasury.claim(first, &watch).await.unwrap();
    assert_eq!(
        TrackClaim::filter_by_track_id(first)
            .exec(&mut db)
            .await
            .unwrap()
            .len(),
        2
    );

    // Letting go of every claim on the first track makes it an orphan.
    for claim in TrackClaim::filter_by_track_id(first)
        .exec(&mut db)
        .await
        .unwrap()
    {
        if claim.kind == ClaimKind::Offering {
            claim.delete().exec(&mut db).await.unwrap();
        }
    }
    let released = hoard
        .treasury
        .release(ClaimKind::WatchPlaylist, "7", |_| false)
        .await
        .unwrap();
    assert_eq!(released, [first]);
    let orphans: Vec<u64> = hoard
        .treasury
        .orphans()
        .await
        .unwrap()
        .iter()
        .map(|t| t.id)
        .collect();
    assert_eq!(orphans, [first]);

    // Claimed tracks are never deleted, whatever is asked.
    assert_eq!(
        hoard
            .treasury
            .delete_orphans(&[first, second])
            .await
            .unwrap(),
        1
    );
    assert!(!first_path.exists());
    assert!(
        Track::filter_by_id(first)
            .first()
            .exec(&mut db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        Track::filter_by_id(second)
            .first()
            .exec(&mut db)
            .await
            .unwrap()
            .is_some()
    );
    // The album still has a track, so it stays, cover and all.
    let album = &Album::all().exec(&mut db).await.unwrap()[0];
    let cover = hoard.treasury.resolve(album.cover.as_deref().unwrap());
    assert!(cover.is_file());

    // The last track takes its album, cover, artist and directories along.
    for claim in TrackClaim::filter_by_track_id(second)
        .exec(&mut db)
        .await
        .unwrap()
    {
        claim.delete().exec(&mut db).await.unwrap();
    }
    assert_eq!(hoard.treasury.delete_orphans(&[second]).await.unwrap(), 1);
    assert!(Album::all().exec(&mut db).await.unwrap().is_empty());
    assert!(Artist::all().exec(&mut db).await.unwrap().is_empty());
    assert!(!cover.exists());
    let treasure = hoard.dir.path().join("treasure");
    assert!(
        !treasure.join("Test Artist").exists(),
        "empty directories remain"
    );
    assert!(hoard.treasury.orphans().await.unwrap().is_empty());
}
