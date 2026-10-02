//! Tests against the real YouTube Music, and Deezer with Monochrome. They
//! need network access and are skipped by default:
//! `cargo test -p pixiu-hunt -- --ignored`.
//!
//! They use a Creative Commons track (Kevin MacLeod, CC BY) so no rights are
//! at stake.

use pixiu_core::SecretBox;
use pixiu_db::{Platform, Track};
use pixiu_hunt::{
    DeezerSource, DownloadRequest, Hunter, Platforms, YouTubeMusicSource, YtMusic, YtMusicPool,
    deezer::MONOCHROME_API,
};
use pixiu_treasury::{Claim, Treasury, tags};

const QUERY: &str = "Kevin MacLeod Monkeys Spinning Monkeys";

fn botguard() -> Option<std::path::PathBuf> {
    std::env::var_os("PIXIU_BOTGUARD").map(Into::into)
}

#[tokio::test]
#[ignore = "needs network access to YouTube Music"]
async fn searches_youtube_music() {
    let dir = tempfile::tempdir().unwrap();
    let ytm = YtMusic::new(dir.path(), SecretBox::ephemeral(), botguard()).unwrap();

    let results = ytm.search(QUERY).await.unwrap();
    let track = results.tracks.first().expect("a track");
    println!("{track:#?}");
    assert!(track.title.to_lowercase().contains("monkeys"));
    assert!(!track.artists.is_empty());
}

#[tokio::test]
#[ignore = "needs network access to YouTube Music"]
async fn downloads_a_track_into_the_treasure() {
    let dir = tempfile::tempdir().unwrap();
    let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    let treasury = Treasury::new(
        db.clone(),
        dir.path().join("treasure"),
        dir.path().join("cache"),
    );
    let ytm = YtMusicPool::new(
        &dir.path().join("ytm"),
        dir.path().join("users"),
        SecretBox::ephemeral(),
        botguard(),
    )
    .unwrap();
    let source = YouTubeMusicSource::new(std::sync::Arc::new(ytm));
    let hunter = Hunter::new(
        Platforms::new([std::sync::Arc::new(source) as _]),
        treasury.clone(),
        dir.path().join("staging"),
    )
    .unwrap();

    let found = hunter
        .platforms()
        .get(Platform::YouTubeMusic)
        .unwrap()
        .search(QUERY)
        .await
        .unwrap();
    let key = found.tracks.first().expect("a track").id.clone();

    let request = DownloadRequest {
        owner: 1,
        job_id: 1,
        key: key.clone(),
        claim: Claim::offering(),
        cookies: None,
    };
    let track: Track = hunter
        .download(&request, &|percent| println!("{percent}%"))
        .await
        .unwrap();
    println!("{track:#?}");

    assert_eq!(track.source_key, Some(key.as_stored()));
    let path = treasury.resolve(&track.path);
    assert!(path.is_file());
    let info = tags::read(&path).unwrap();
    assert!(info.title.unwrap().to_lowercase().contains("monkeys"));

    // Someone else grabbing the same video shares the stored file.
    let theirs = hunter
        .download(
            &DownloadRequest {
                owner: 2,
                job_id: 2,
                ..request.clone()
            },
            &|_| {},
        )
        .await
        .unwrap();
    assert_eq!(theirs.file_id, track.file_id);
    assert_ne!(theirs.album_id, track.album_id);
    assert!(info.duration_ms > 60_000, "{} ms", info.duration_ms);
    assert!(info.cover.is_some(), "the cover is embedded");
    assert!(
        std::fs::read_dir(hunter.staging())
            .unwrap()
            .next()
            .is_none(),
        "staging is cleaned up"
    );

    // Downloading it again is refused.
    let again = hunter.download(&request, &|_| {}).await.unwrap_err();
    assert!(matches!(
        again,
        pixiu_hunt::HuntError::AlreadyHoarded { .. }
    ));
}

#[tokio::test]
#[ignore = "needs network access to YouTube Music"]
async fn reads_playlists_and_discographies() {
    let dir = tempfile::tempdir().unwrap();
    let ytm = YtMusic::new(dir.path(), SecretBox::ephemeral(), botguard()).unwrap();

    // An album's playlist: The August Album (Kevin MacLeod, CC BY).
    let artist = ytm.search("Kevin MacLeod").await.unwrap().artists.remove(0);
    let discography = ytm.discography(artist.id.id()).await.unwrap();
    println!("{} releases", discography.albums.len());
    assert_eq!(discography.name, "Kevin MacLeod");
    assert!(discography.albums.len() > 10);
    assert!(
        discography
            .albums
            .iter()
            .any(|album| album.kind == pixiu_hunt::AlbumKind::Single)
    );

    let album = discography
        .albums
        .iter()
        .find(|album| album.title == "The August Album")
        .expect("The August Album");
    let listed = ytm.album(album.id.id()).await.unwrap();
    assert_eq!(listed.tracks.len(), 4);

    // Watching this public playlist needs no login; liked music does.
    let playlist = ytm
        .playlist("OLAK5uy_kJpzyLJxwhFJ8eZrw8XkMNoMl8BGr0tPs")
        .await
        .unwrap();
    println!("{playlist:#?}");
    assert_eq!(playlist.tracks.len(), 1);
    let liked = ytm.playlist(pixiu_hunt::LIKED_MUSIC).await.unwrap_err();
    assert!(liked.needs_login(), "{liked}");
}

#[tokio::test]
#[ignore = "needs network access to Deezer and Monochrome"]
async fn downloads_a_deezer_song_through_monochrome() {
    let dir = tempfile::tempdir().unwrap();
    let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    let treasury = Treasury::new(db, dir.path().join("treasure"), dir.path().join("cache"));
    let deezer = DeezerSource::new(MONOCHROME_API).unwrap();
    let hunter = Hunter::new(
        Platforms::new([std::sync::Arc::new(deezer) as _]),
        treasury.clone(),
        dir.path().join("staging"),
    )
    .unwrap();
    let deezer = hunter.platforms().get(Platform::Deezer).unwrap();

    let found = deezer.search(QUERY).await.unwrap();
    let song = found.tracks.first().expect("a song").clone();
    println!("{song:#?}");
    assert!(song.title.to_lowercase().contains("monkeys"));
    assert!(song.isrc.is_some(), "Deezer names ISRCs");
    let album = deezer
        .album(song.album.as_ref().expect("an album").id.id())
        .await
        .unwrap();
    assert!(album.tracks.iter().any(|track| track.id == song.id));
    let artist = deezer
        .discography(song.artist_id.as_ref().expect("an artist").id())
        .await
        .unwrap();
    assert!(artist.albums.len() > 10, "{} releases", artist.albums.len());

    let track: Track = hunter
        .download(
            &DownloadRequest {
                owner: 1,
                job_id: 1,
                key: song.id.clone(),
                claim: Claim::offering(),
                cookies: None,
            },
            &|percent| println!("{percent}%"),
        )
        .await
        .unwrap();
    assert_eq!(track.suffix, "opus");
    let info = tags::read(&treasury.resolve(&track.path)).unwrap();
    println!("{info:#?}");
    assert_eq!(info.isrc, song.isrc);
    assert!(info.duration_ms > 60_000);
}
