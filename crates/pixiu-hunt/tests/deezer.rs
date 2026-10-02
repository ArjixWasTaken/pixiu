//! Deezer downloads, with fake Deezer and Monochrome APIs on local HTTP:
//! what Deezer says about a song, how Monochrome's stream is found, and
//! what gets filed.

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use pixiu_db::{Album, SourceKey, Track};
use pixiu_hunt::{
    DeezerSource, DownloadRequest, HuntError, Hunter, Platforms, Source, model::AlbumKind,
};
use pixiu_treasury::{Claim, Treasury, tags};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

fn fixture(name: &str) -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/audio")
        .join(name)
}

const ISRC: &str = "ZZXX12100003";

/// A reply: status, content type, extra headers and body.
type Reply = (u16, &'static str, Vec<String>, Vec<u8>);

fn json(body: &str) -> Reply {
    (
        200,
        "application/json",
        Vec::new(),
        body.as_bytes().to_vec(),
    )
}

/// What the fake servers answer to `method` on `target` (path and query).
fn reply(base: &str, method: &str, target: &str, range: Option<(u64, u64)>) -> Reply {
    let audio = std::fs::read(fixture("01-first-light.flac")).unwrap();
    let song = |id: u64, title: &str, isrc: &str| {
        format!(
            r#"{{"id": {id}, "title": "{title}", "duration": 1, "isrc": "{isrc}",
                "track_position": 3, "disk_number": 2,
                "artist": {{"id": 7, "name": "Main Artist"}},
                "contributors": [{{"id": 7, "name": "Main Artist", "role": "Main"}},
                                 {{"id": 8, "name": "Guest", "role": "Featured"}}],
                "album": {{"id": 5, "title": "Morning", "cover_xl": "{base}/cover.jpg"}}}}"#
        )
    };
    match (method, target) {
        // Deezer.
        ("GET", "/deezer/track/42") => json(&song(42, "Dawn Chorus", ISRC)),
        ("GET", "/deezer/track/43") => json(&song(43, "Unheard", "ZZXX12100099")),
        ("GET", "/deezer/album/5") => json(&format!(
            r#"{{"id": 5, "title": "Morning", "release_date": "2021-04-02",
                "record_type": "album", "cover_xl": "{base}/cover.jpg",
                "artist": {{"id": 7, "name": "Main Artist"}}}}"#
        )),
        ("GET", "/deezer/album/5/tracks?limit=100") => json(&format!(
            r#"{{"data": [{{"id": 41, "title": "Overture", "duration": 60, "isrc": "ZZXX12100001",
                           "track_position": 1, "disk_number": 1,
                           "artist": {{"id": 7, "name": "Main Artist"}}}}],
                "next": "{base}/deezer/album/5/tracks?limit=100&index=1"}}"#
        )),
        ("GET", "/deezer/album/5/tracks?limit=100&index=1") => json(
            r#"{"data": [{"id": 42, "title": "Dawn Chorus", "duration": 1, "isrc": "ZZXX12100003",
                          "track_position": 3, "disk_number": 2,
                          "artist": {"id": 7, "name": "Main Artist"}}]}"#,
        ),
        // Monochrome.
        ("GET", target) if target.starts_with("/monochrome/search/tracks?q=Main+Artist+Dawn") => {
            json(&format!(
                r#"{{"tracks": [
                    {{"trackId": "900", "title": "Dawn Chorus", "artistNames": ["Someone Else"],
                      "duration": 1000, "isrc": "ZZXX00000000", "playable": true}},
                    {{"trackId": "901", "title": "Dawn Chorus (Remastered)", "artistNames": ["Main Artist"],
                      "duration": 1000, "isrc": "{ISRC}", "playable": true}}]}}"#
            ))
        }
        ("GET", target) if target.starts_with("/monochrome/search/tracks") => {
            json(r#"{"tracks": []}"#)
        }
        ("HEAD", "/monochrome/track/901") => (
            200,
            "application/octet-stream",
            vec![format!("Content-Length: {}", audio.len())],
            Vec::new(),
        ),
        ("GET", "/monochrome/track/901") => match range {
            Some((start, end)) => {
                let total = audio.len();
                let end = usize::try_from(end).unwrap().min(total - 1);
                let start = usize::try_from(start).unwrap();
                (
                    206,
                    "application/octet-stream",
                    vec![format!("Content-Range: bytes {start}-{end}/{total}")],
                    audio[start..=end].to_vec(),
                )
            }
            None => (200, "application/octet-stream", Vec::new(), audio),
        },
        ("GET", "/cover.jpg") => (
            200,
            "image/jpeg",
            Vec::new(),
            std::fs::read(fixture("folder.jpg")).unwrap(),
        ),
        _ => json(r#"{"error": {"type": "DataException", "message": "no data", "code": 800}}"#),
    }
}

/// Serves [`reply`], counting requests for Monochrome's streams.
async fn serve() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let streams = Arc::new(AtomicUsize::new(0));
    let (address, counter) = (base.clone(), Arc::clone(&streams));
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            let (base, counter) = (address.clone(), Arc::clone(&counter));
            tokio::spawn(async move {
                let mut request = vec![0_u8; 8192];
                let read = stream.read(&mut request).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&request[..read]).into_owned();
                let mut words = request.split_whitespace();
                let (method, target) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
                let range = request.lines().find_map(|line| {
                    let (start, end) = line
                        .strip_prefix("range: bytes=")
                        .or_else(|| line.strip_prefix("Range: bytes="))?
                        .split_once('-')?;
                    Some((start.parse().ok()?, end.trim().parse().ok()?))
                });
                if method == "GET" && target.starts_with("/monochrome/track/") {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
                let (status, content_type, headers, body) = reply(&base, method, target, range);
                let mut head = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Type: {content_type}\r\nConnection: close\r\n"
                );
                if !headers
                    .iter()
                    .any(|header| header.starts_with("Content-Length"))
                {
                    head.push_str(&format!("Content-Length: {}\r\n", body.len()));
                }
                for header in headers {
                    head.push_str(&header);
                    head.push_str("\r\n");
                }
                head.push_str("\r\n");
                let _ = stream.write_all(head.as_bytes()).await;
                let _ = stream.write_all(&body).await;
            });
        }
    });
    (base, streams)
}

struct Setup {
    _dir: tempfile::TempDir,
    hunter: Hunter,
    treasury: Treasury,
    streams: Arc<AtomicUsize>,
}

async fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    let treasury = Treasury::new(db, dir.path().join("treasure"), dir.path().join("cache"));
    let (base, streams) = serve().await;
    let deezer =
        DeezerSource::with_api(&format!("{base}/deezer"), &format!("{base}/monochrome")).unwrap();
    let platforms = Platforms::new([Arc::new(deezer) as Arc<dyn Source>]);
    let hunter = Hunter::new(platforms, treasury.clone(), dir.path().join("staging")).unwrap();
    Setup {
        _dir: dir,
        hunter,
        treasury,
        streams,
    }
}

async fn download(s: &Setup, id: &str) -> Result<Track, HuntError> {
    let request = DownloadRequest {
        owner: 1,
        job_id: 100,
        key: SourceKey::deezer(id),
        claim: Claim::offering(),
        cookies: None,
    };
    s.hunter.download(&request, &|_| {}).await
}

#[tokio::test]
async fn deezer_songs_come_from_monochrome_as_opus() {
    let s = setup().await;

    let track = download(&s, "42").await.unwrap();

    assert_eq!(track.source_key.as_deref(), Some("deezer:42"));
    assert_eq!(track.title, "Dawn Chorus");
    assert_eq!(track.artist_credit, "Main Artist, Guest");
    assert_eq!((track.track_number, track.disc_number), (Some(3), Some(2)));
    assert_eq!(track.isrc.as_deref(), Some(ISRC));
    assert_eq!(
        track.suffix, "opus",
        "lossless downloads are stored as Opus"
    );
    let mut db = s.treasury.db();
    let album = Album::get_by_id(&mut db, &track.album_id).await.unwrap();
    assert_eq!(album.title, "Morning");
    assert_eq!(album.year, Some(2021));
    assert_eq!(album.source_key.as_deref(), Some("deezer:5"));

    let file = tags::read(&s.treasury.resolve(&track.path)).unwrap();
    assert_eq!(file.isrc.as_deref(), Some(ISRC));
    assert_eq!(file.disc_number, Some(2));
    assert!(file.cover.is_some(), "the album's cover is in the file");
    assert_eq!(s.streams.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn songs_monochrome_lacks_say_so() {
    let s = setup().await;

    let error = download(&s, "43").await.unwrap_err();

    assert!(
        error.to_string() == "download failed: Monochrome doesn't have this song",
        "{error}"
    );
    assert_eq!(s.streams.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn songs_deezer_lacks_say_so() {
    let s = setup().await;

    let error = download(&s, "404").await.unwrap_err();

    assert!(
        error.to_string().contains("Deezer has no such song"),
        "{error}"
    );
}

#[tokio::test]
async fn albums_come_with_every_page_of_songs() {
    let s = setup().await;

    let album = s.hunter.album(&SourceKey::deezer("5")).await.unwrap();

    assert_eq!(album.title, "Morning");
    assert_eq!(album.kind, AlbumKind::Album);
    let songs: Vec<_> = album
        .tracks
        .iter()
        .map(|track| (track.id.id(), track.track_number, track.disc_number))
        .collect();
    assert_eq!(songs, [("41", Some(1), Some(1)), ("42", Some(3), Some(2))]);
    assert!(
        album
            .tracks
            .iter()
            .all(|track| track.album.as_ref().map(|a| a.id.id()) == Some("5")),
        "songs listed by their album know it"
    );
}
