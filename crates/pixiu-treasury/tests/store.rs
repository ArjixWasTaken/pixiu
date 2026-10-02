//! The store: files kept once, released when unused, and the adoption of
//! libraries filed before it existed.

use std::path::{Path, PathBuf};

use pixiu_db::{
    Album, Artist, AudioFile, ClaimKind, Db, Track, TrackClaim, TrackOrigin, now, toasty,
};
use pixiu_treasury::{AlbumEdit, ArtistRef, Claim, Cover, Provenance, Treasury, store, tags};

/// Every test library belongs to one user.
const OWNER: u64 = 1;

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

    fn treasure(&self, relative: &str) -> PathBuf {
        self.dir.path().join("treasure").join(relative)
    }

    /// Puts a fixture at `relative` inside the treasure, like the old
    /// layout did.
    fn place(&self, fixture_name: &str, relative: &str) -> PathBuf {
        let path = self.treasure(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::copy(fixture(fixture_name), &path).unwrap();
        path
    }

    async fn offer(&self, name: &str) -> Track {
        self.offer_as(OWNER, name).await
    }

    async fn offer_as(&self, owner: u64, name: &str) -> Track {
        let staging = self.dir.path().join("staging");
        std::fs::create_dir_all(&staging).unwrap();
        let staged = staging.join(name);
        std::fs::copy(fixture(name), &staged).unwrap();
        let info = tags::read(&staged).unwrap();
        self.treasury
            .ingest(
                owner,
                &staged,
                &info,
                None,
                Provenance::offering(name, None),
                Claim::offering(),
            )
            .await
            .unwrap()
    }

    /// A track as filed before the store existed: no file id, and its path
    /// in the old layout.
    async fn legacy_track(&self, album: &Album, title: &str, path: &str) -> Track {
        let mut db = self.db.clone();
        let track = toasty::create!(Track {
            user_id: album.user_id,
            album_id: album.id,
            artist_id: album.artist_id,
            title,
            artist_credit: "Old Artist",
            duration_ms: 1000,
            file_id: 0,
            path,
            size: 1,
            suffix: "flac",
            content_type: "audio/flac",
            source_key: Some(format!("youtube_music:vid-{title}")),
            origin: TrackOrigin::Download,
            added_at: now(),
        })
        .exec(&mut db)
        .await
        .unwrap();
        toasty::create!(TrackClaim {
            track_id: track.id,
            kind: ClaimKind::ManualGrab,
            created_at: now(),
        })
        .exec(&mut db)
        .await
        .unwrap();
        track
    }
}

fn sha256(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(std::fs::read(path).unwrap()))
}

#[tokio::test]
async fn unused_stored_files_are_collected() {
    let hoard = Hoard::new().await;
    let mut db = hoard.db.clone();
    let track = hoard.offer("01-first-light.flac").await;

    // Left by a crash: a file no row names, a row no track plays, and half
    // a copy.
    let stray = hoard.treasure(&store::audio_path(&"cd".repeat(32), "mp3"));
    std::fs::create_dir_all(stray.parent().unwrap()).unwrap();
    std::fs::write(&stray, b"stray").unwrap();
    let unplayed = hoard.place(
        "02-second-wind.mp3",
        &store::audio_path(&"ef".repeat(32), "mp3"),
    );
    toasty::create!(AudioFile {
        sha256: "ef".repeat(32),
        path: store::audio_path(&"ef".repeat(32), "mp3"),
        size: 1,
        suffix: "mp3",
        content_type: "audio/mpeg",
        duration_ms: 1,
        created_at: now(),
    })
    .exec(&mut db)
    .await
    .unwrap();
    let partial = hoard.treasure(".store/images/ab/.abab.jpg.tmp");
    std::fs::create_dir_all(partial.parent().unwrap()).unwrap();
    std::fs::write(&partial, b"half").unwrap();

    assert_eq!(hoard.treasury.collect_garbage().await.unwrap(), 3);
    assert!(!stray.exists() && !unplayed.exists() && !partial.exists());
    assert_eq!(AudioFile::all().exec(&mut db).await.unwrap().len(), 1);
    // What is used stays.
    assert!(hoard.treasury.resolve(&track.path).is_file());
    let album = &Album::all().exec(&mut db).await.unwrap()[0];
    assert!(
        hoard
            .treasury
            .resolve(album.cover.as_deref().unwrap())
            .is_file()
    );
    assert_eq!(hoard.treasury.collect_garbage().await.unwrap(), 0);
}

#[tokio::test]
async fn edits_leave_stored_files_alone() {
    let hoard = Hoard::new().await;
    let mut db = hoard.db.clone();
    let track = hoard.offer("01-first-light.flac").await;
    let file = hoard.treasury.resolve(&track.path);
    let before = sha256(&file);

    hoard
        .treasury
        .edit_album(&AlbumEdit {
            album_id: track.album_id,
            title: "Renamed".into(),
            artist: ArtistRef {
                name: "Someone Else".into(),
                mbid: None,
            },
            year: Some(1999),
            mbid: None,
            rg_mbid: None,
            tracks: Vec::new(),
        })
        .await
        .unwrap();
    let album = Album::get_by_id(&mut db, &track.album_id).await.unwrap();
    assert_eq!((album.title.as_str(), album.year), ("Renamed", Some(1999)));
    let after = Track::get_by_id(&mut db, &track.id).await.unwrap();
    assert_eq!(after.path, track.path);
    assert_eq!(sha256(&file), before, "the file was not rewritten");

    // A new cover replaces the old picture, which nothing else shows.
    let old = hoard.treasury.resolve(album.cover.as_deref().unwrap());
    let png = std::fs::read(fixture("folder.jpg")).unwrap();
    hoard
        .treasury
        .replace_cover(
            album.id,
            &Cover {
                data: png,
                mime: "image/jpeg".into(),
            },
        )
        .await
        .unwrap();
    let album = Album::get_by_id(&mut db, &track.album_id).await.unwrap();
    let new = hoard.treasury.resolve(album.cover.as_deref().unwrap());
    assert!(new.is_file() && new != old);
    assert!(!old.exists());
    assert_eq!(sha256(&file), before);
}

#[tokio::test]
async fn legacy_files_are_adopted_once() {
    let hoard = Hoard::new().await;
    let mut db = hoard.db.clone();
    let artist = toasty::create!(Artist {
        user_id: OWNER,
        name: "Old Artist",
        name_key: "old artist",
        image: Some("artists/1.png".to_owned()),
        created_at: now(),
    })
    .exec(&mut db)
    .await
    .unwrap();
    let album = toasty::create!(Album {
        user_id: OWNER,
        title: "Old Album",
        title_key: "old album",
        artist_id: artist.id,
        cover: Some("Old Artist/Old Album/cover.jpg".to_owned()),
        created_at: now(),
        single: false,
    })
    .exec(&mut db)
    .await
    .unwrap();

    let first = hoard.place("01-first-light.flac", "Old Artist/Old Album/01 First.flac");
    let content = sha256(&first);
    hoard.place("01-first-light.flac", "Old Artist/Old Album/02 Same.flac");
    let first_track = hoard
        .legacy_track(&album, "First", "Old Artist/Old Album/01 First.flac")
        .await;
    let same_track = hoard
        .legacy_track(&album, "Same", "Old Artist/Old Album/02 Same.flac")
        .await;
    let lost_track = hoard
        .legacy_track(&album, "Lost", "Old Artist/Old Album/03 Lost.flac")
        .await;
    hoard.place("folder.jpg", "Old Artist/Old Album/cover.jpg");
    let picture = hoard.dir.path().join("cache/artists/1.png");
    std::fs::create_dir_all(picture.parent().unwrap()).unwrap();
    let png = tags::read(&fixture("01-first-light.flac"))
        .unwrap()
        .cover
        .unwrap();
    std::fs::write(&picture, &png.data).unwrap();
    // A leftover copy of stored content, and something unrelated.
    hoard.place("01-first-light.flac", "Old Artist/Elsewhere/copy.flac");
    let notes = hoard.treasure("Old Artist/notes.txt");
    std::fs::write(&notes, "keep me").unwrap();
    // A crash once linked the file into the store without recording it.
    let crashed = hoard.treasure(&store::audio_path(&content, "flac"));
    std::fs::create_dir_all(crashed.parent().unwrap()).unwrap();
    std::fs::hard_link(&first, &crashed).unwrap();

    let adoption = hoard.treasury.adopt_legacy().await.unwrap();
    assert_eq!(
        adoption,
        pixiu_treasury::Adoption {
            files: 1,
            duplicates: 1,
            missing: 1,
            pictures: 2,
            swept: 1,
            kept: 1,
        }
    );

    let files = AudioFile::all().exec(&mut db).await.unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].sha256, content);
    assert_eq!(
        files[0].source_key.as_deref(),
        Some("youtube_music:vid-First")
    );
    for track in [&first_track, &same_track] {
        let track = Track::get_by_id(&mut db, &track.id).await.unwrap();
        assert_eq!((track.file_id, &track.path), (files[0].id, &files[0].path));
        assert_eq!(sha256(&hoard.treasury.resolve(&track.path)), content);
    }
    // A track whose file is missing is left as it was.
    let lost = Track::get_by_id(&mut db, &lost_track.id).await.unwrap();
    assert_eq!(
        (lost.file_id, lost.path.as_str()),
        (0, "Old Artist/Old Album/03 Lost.flac")
    );

    let album = Album::get_by_id(&mut db, &album.id).await.unwrap();
    let cover = album.cover.unwrap();
    assert!(cover.starts_with(".store/images/"));
    assert!(hoard.treasury.resolve(&cover).is_file());
    let artist = Artist::get_by_id(&mut db, &artist.id).await.unwrap();
    let image = artist.image.unwrap();
    assert!(image.starts_with(".store/images/") && image.ends_with(".png"));
    assert!(!picture.exists());

    // The old folders keep only what the library does not know.
    assert!(notes.is_file());
    assert!(!hoard.treasure("Old Artist/Old Album").exists());
    assert!(!hoard.treasure("Old Artist/Elsewhere").exists());

    // Again: nothing left to do.
    assert_eq!(
        hoard.treasury.adopt_legacy().await.unwrap(),
        pixiu_treasury::Adoption {
            missing: 1,
            ..Default::default()
        }
    );
    assert_eq!(hoard.treasury.collect_garbage().await.unwrap(), 0);
}

#[tokio::test]
async fn libraries_share_stored_files() {
    let hoard = Hoard::new().await;
    let mut db = hoard.db.clone();
    let mine = hoard.offer_as(1, "01-first-light.flac").await;
    let theirs = hoard.offer_as(2, "01-first-light.flac").await;

    // Two tracks in two libraries, one file.
    assert_ne!(mine.id, theirs.id);
    assert_ne!(mine.album_id, theirs.album_id);
    assert_eq!(mine.file_id, theirs.file_id);
    assert_eq!(AudioFile::all().exec(&mut db).await.unwrap().len(), 1);
    let file = hoard.treasury.resolve(&mine.path);
    let cover = hoard.treasury.resolve(
        Album::get_by_id(&mut db, &mine.album_id)
            .await
            .unwrap()
            .cover
            .as_deref()
            .unwrap(),
    );

    // Nobody deletes another library's tracks.
    for track in [&mine, &theirs] {
        TrackClaim::filter_by_track_id(track.id)
            .delete()
            .exec(&mut db)
            .await
            .unwrap();
    }
    assert_eq!(hoard.treasury.orphans(2).await.unwrap().len(), 1);
    assert_eq!(
        hoard.treasury.delete_orphans(2, &[mine.id]).await.unwrap(),
        0
    );

    // The file stays while anyone plays it, and the cover while anyone
    // shows it.
    assert_eq!(
        hoard
            .treasury
            .delete_orphans(2, &[theirs.id])
            .await
            .unwrap(),
        1
    );
    assert!(file.is_file() && cover.is_file());
    assert_eq!(
        hoard.treasury.delete_orphans(1, &[mine.id]).await.unwrap(),
        1
    );
    assert!(!file.exists() && !cover.exists());
    assert!(AudioFile::all().exec(&mut db).await.unwrap().is_empty());
}
