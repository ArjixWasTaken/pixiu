//! Downloads, with a fake platform serving a fixture over local HTTP: what
//! a download files, what it skips, and what it reuses.

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use pixiu_db::{Platform, SourceKey, Track};
use pixiu_hunt::{
    AudioSource, Discography, DownloadRequest, HuntError, Hunter, Link, Page, Platforms,
    RemoteAlbum, RemotePlaylist, RemoteTrack, SearchResults, Source, YtDlpTarget,
    model::{AlbumKind, AlbumRef},
    source::BoxFuture,
};
use pixiu_treasury::{Claim, Treasury};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

fn fixture(name: &str) -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/audio")
        .join(name)
}

fn key(id: &str) -> SourceKey {
    SourceKey::youtube_music(id)
}

/// Serves the fixture to every request, counting them.
async fn serve_audio() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let served = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&served);
    let body = std::fs::read(fixture("untagged.opus")).unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            counter.fetch_add(1, Ordering::SeqCst);
            let body = body.clone();
            tokio::spawn(async move {
                let mut request = [0_u8; 4096];
                let _ = stream.read(&mut request).await;
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: audio/ogg\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes()).await;
                let _ = stream.write_all(&body).await;
            });
        }
    });
    (address, served)
}

/// A platform with one album, whose songs stream from the local server;
/// the song `broken` has no stream.
struct FakeSource {
    audio_url: String,
}

fn song(id: &str) -> RemoteTrack {
    RemoteTrack {
        id: key(id),
        title: format!("Song {id}"),
        artists: vec!["Fake Artist".to_owned()],
        artist_id: Some(key("UCfake")),
        album: Some(AlbumRef {
            id: key("MPREb_fake"),
            title: "Fake Album".to_owned(),
        }),
        duration_secs: Some(1),
        track_number: Some(1),
        disc_number: None,
        isrc: None,
        cover_url: None,
        is_video: false,
    }
}

fn unsupported<T>() -> Result<T, HuntError> {
    Err(HuntError::NoSource(Platform::YouTubeMusic))
}

impl Source for FakeSource {
    fn platform(&self) -> Platform {
        Platform::YouTubeMusic
    }

    fn parse_link(&self, _input: &str) -> Option<Link> {
        None
    }

    fn search<'a>(&'a self, _query: &'a str) -> BoxFuture<'a, Result<SearchResults, HuntError>> {
        Box::pin(async { Ok(SearchResults::default()) })
    }

    fn track<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<RemoteTrack, HuntError>> {
        Box::pin(async move { Ok(song(id)) })
    }

    fn album<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<RemoteAlbum, HuntError>> {
        Box::pin(async move {
            Ok(RemoteAlbum {
                id: key(id),
                title: "Fake Album".to_owned(),
                artists: vec!["Fake Artist".to_owned()],
                artist_id: Some(key("UCfake")),
                year: Some(2026),
                kind: AlbumKind::Album,
                cover_url: None,
                tracks: vec![song("one")],
            })
        })
    }

    fn playlist<'a>(
        &'a self,
        _owner: u64,
        _id: &'a str,
    ) -> BoxFuture<'a, Result<RemotePlaylist, HuntError>> {
        Box::pin(async { unsupported() })
    }

    fn discography<'a>(&'a self, _id: &'a str) -> BoxFuture<'a, Result<Discography, HuntError>> {
        Box::pin(async { unsupported() })
    }

    fn lyrics<'a>(
        &'a self,
        _id: &'a str,
    ) -> BoxFuture<'a, Result<Option<(String, String)>, HuntError>> {
        Box::pin(async { Ok(None) })
    }

    fn audio<'a>(
        &'a self,
        _owner: u64,
        id: &'a str,
    ) -> BoxFuture<'a, Result<AudioSource, HuntError>> {
        Box::pin(async move {
            if id == "broken" {
                return Err(HuntError::NoAudio(id.to_owned()));
            }
            Ok(AudioSource {
                url: format!("{}/{id}", self.audio_url),
                size: None,
                user_agent: "pixiu-test".to_owned(),
                is_webm_opus: false,
                extension: "opus",
            })
        })
    }

    fn page_url(&self, _page: Page, id: &str) -> String {
        format!("https://example.com/{id}")
    }

    fn yt_dlp(&self, _id: &str) -> Option<YtDlpTarget> {
        None
    }
}

struct Setup {
    _dir: tempfile::TempDir,
    hunter: Hunter,
    treasury: Treasury,
    served: Arc<AtomicUsize>,
}

async fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    let treasury = Treasury::new(db, dir.path().join("treasure"), dir.path().join("cache"));
    let (audio_url, served) = serve_audio().await;
    let platforms = Platforms::new([Arc::new(FakeSource { audio_url }) as Arc<dyn Source>]);
    let hunter = Hunter::new(platforms, treasury.clone(), dir.path().join("staging")).unwrap();
    Setup {
        _dir: dir,
        hunter,
        treasury,
        served,
    }
}

fn request(owner: u64, id: &str) -> DownloadRequest {
    DownloadRequest {
        owner,
        job_id: owner * 100,
        key: key(id),
        claim: Claim::offering(),
        cookies: None,
    }
}

async fn download(s: &Setup, owner: u64, id: &str) -> Result<Track, HuntError> {
    s.hunter.download(&request(owner, id), &|_| {}).await
}

#[tokio::test]
async fn songs_are_downloaded_tagged_and_filed() {
    let s = setup().await;

    let track = download(&s, 1, "one").await.unwrap();

    assert_eq!(track.source_key.as_deref(), Some("youtube_music:one"));
    assert_eq!(track.title, "Song one");
    assert_eq!(track.artist_credit, "Fake Artist");
    assert_eq!(track.suffix, "opus");
    assert!(s.treasury.resolve(&track.path).is_file());
    let mut db = s.treasury.db();
    let album = pixiu_db::Album::get_by_id(&mut db, &track.album_id)
        .await
        .unwrap();
    assert_eq!(album.title, "Fake Album");
    assert_eq!(
        album.source_key.as_deref(),
        Some("youtube_music:MPREb_fake")
    );
    assert_eq!(s.served.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_song_held_already_is_not_fetched_again() {
    let s = setup().await;
    let first = download(&s, 1, "one").await.unwrap();

    let again = download(&s, 1, "one").await.unwrap_err();

    assert!(matches!(again, HuntError::AlreadyHoarded { track_id } if track_id == first.id));
    assert_eq!(s.served.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn another_library_shares_the_stored_file() {
    let s = setup().await;
    let mine = download(&s, 1, "one").await.unwrap();

    let theirs = download(&s, 2, "one").await.unwrap();

    assert_eq!(theirs.user_id, 2);
    assert_eq!(theirs.file_id, mine.file_id);
    assert_eq!(theirs.source_key.as_deref(), Some("youtube_music:one"));
    // Nothing downloaded the second time.
    assert_eq!(s.served.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn without_yt_dlp_to_fall_back_on_the_platform_s_reason_stands() {
    let s = setup().await;

    let error = download(&s, 1, "broken").await.unwrap_err();

    // The fake platform names no target for yt-dlp.
    assert!(
        matches!(&error, HuntError::NoAudio(id) if id == "broken"),
        "{error}"
    );
    assert_eq!(s.served.load(Ordering::SeqCst), 0);
}
