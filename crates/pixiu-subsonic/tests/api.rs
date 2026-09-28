//! The Subsonic API against a small real library.

use std::path::{Path, PathBuf};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, Method, Request, StatusCode, header},
};
use md5::{Digest, Md5};
use pixiu_core::SecretBox;
use pixiu_db::{
    ApiKey, ClaimKind, Db, Playlist, PlaylistEntry, Track, TrackClaim, User, now, toasty,
};
use pixiu_subsonic::SubsonicState;
use pixiu_treasury::{Claim, Provenance, Treasury, tags};
use serde_json::Value;
use tower::ServiceExt;

const PASSWORD: &str = "gold-and-jade";
const API_KEY: &str = "pixiu_test_key";
/// Credentials as clients send them, with plain password auth.
const AUTH: &str = "u=keeper&p=gold-and-jade&v=1.16.1&c=test&f=json";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/audio")
        .join(name)
}

struct Api {
    _dir: tempfile::TempDir,
    db: Db,
    router: Router,
}

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl Reply {
    fn json(&self) -> Value {
        let value: Value = serde_json::from_slice(&self.body).unwrap_or_else(|error| {
            panic!(
                "not JSON ({error}): {}",
                String::from_utf8_lossy(&self.body)
            )
        });
        value["subsonic-response"].clone()
    }

    fn ok(&self) -> Value {
        let json = self.json();
        assert_eq!(json["status"], "ok", "{json:#}");
        json
    }

    fn error_code(&self) -> i64 {
        let json = self.json();
        assert_eq!(json["status"], "failed", "{json:#}");
        json["error"]["code"].as_i64().unwrap()
    }
}

impl Api {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
        let secrets = SecretBox::ephemeral();
        let treasury = Treasury::new(
            db.clone(),
            dir.path().join("treasure"),
            dir.path().join("cache"),
        );

        let user = toasty::create!(User {
            username: "keeper",
            password_hash: pixiu_core::password::hash(PASSWORD),
            subsonic_secret: Some(secrets.seal_str(PASSWORD)),
            created_at: now(),
        })
        .exec(&mut db)
        .await
        .unwrap();
        toasty::create!(ApiKey {
            user_id: user.id,
            name: "test",
            key_hash: ApiKey::hash(API_KEY),
            created_at: now(),
        })
        .exec(&mut db)
        .await
        .unwrap();

        let staging = dir.path().join("staging");
        std::fs::create_dir_all(&staging).unwrap();
        for name in ["01-first-light.flac", "02-second-wind.mp3", "untagged.opus"] {
            let staged = staging.join(name);
            std::fs::copy(fixture(name), &staged).unwrap();
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
                .unwrap();
        }

        let router = pixiu_subsonic::router(SubsonicState {
            db: db.clone(),
            treasury,
            secrets,
        });
        Self {
            _dir: dir,
            db,
            router,
        }
    }

    async fn request(&self, request: Request<Body>) -> Reply {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec();
        Reply {
            status,
            headers,
            body,
        }
    }

    /// Calls `method` with the default credentials and extra `query`.
    async fn call(&self, method: &str, query: &str) -> Reply {
        self.get(&format!("/rest/{method}?{AUTH}&{query}")).await
    }

    async fn get(&self, uri: &str) -> Reply {
        self.request(Request::get(uri).body(Body::empty()).unwrap())
            .await
    }

    async fn album_id(&self, name: &str) -> String {
        let query = name.replace(' ', "%20");
        let json = self.call("search3", &format!("query={query}")).await.ok();
        json["searchResult3"]["album"][0]["id"]
            .as_str()
            .unwrap()
            .to_owned()
    }
}

fn names(list: &Value, field: &str) -> Vec<String> {
    list.as_array()
        .unwrap_or_else(|| panic!("not a list: {list:#}"))
        .iter()
        .map(|item| item[field].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn authentication_methods() {
    let api = Api::new().await;
    let ping = |query: String| {
        let api = &api;
        async move { api.get(&format!("/rest/ping.view?f=json&{query}")).await }
    };

    assert_eq!(ping(String::new()).await.error_code(), 10);
    assert_eq!(ping("u=keeper&p=wrong".into()).await.error_code(), 40);
    assert_eq!(
        ping("u=thief&p=gold-and-jade".into()).await.error_code(),
        40
    );
    ping("u=keeper&p=gold-and-jade".into()).await.ok();
    ping(format!("u=keeper&p=enc:{}", hex::encode(PASSWORD)))
        .await
        .ok();

    let token = hex::encode(Md5::digest(format!("{PASSWORD}s4lt3d")));
    ping(format!("u=keeper&t={token}&s=s4lt3d")).await.ok();
    assert_eq!(
        ping(format!("u=keeper&t={token}&s=other"))
            .await
            .error_code(),
        40
    );

    ping(format!("apiKey={API_KEY}")).await.ok();
    assert_eq!(ping("apiKey=nope".into()).await.error_code(), 44);
    assert_eq!(
        ping(format!("apiKey={API_KEY}&u=keeper"))
            .await
            .error_code(),
        43
    );

    // Extensions are public.
    let json = api.get("/rest/getOpenSubsonicExtensions?f=json").await.ok();
    assert_eq!(
        names(&json["openSubsonicExtensions"], "name"),
        ["formPost", "apiKeyAuthentication"]
    );
}

#[tokio::test]
async fn token_auth_needs_a_captured_password() {
    let api = Api::new().await;
    let mut db = api.db.clone();
    let mut user = User::get_by_username(&mut db, "keeper").await.unwrap();
    toasty::update!(user {
        subsonic_secret: Option::<String>::None
    })
    .exec(&mut db)
    .await
    .unwrap();

    let token = hex::encode(Md5::digest(format!("{PASSWORD}abcdef")));
    let token_ping = format!("/rest/ping?f=json&u=keeper&t={token}&s=abcdef");
    assert_eq!(api.get(&token_ping).await.error_code(), 41);

    // A plain-password login captures it.
    api.call("ping", "").await.ok();
    api.get(&token_ping).await.ok();
}

#[tokio::test]
async fn browsing_the_library() {
    let api = Api::new().await;

    let json = api.call("getMusicFolders", "").await.ok();
    assert_eq!(json["musicFolders"]["musicFolder"][0]["id"], 1);

    let json = api.call("getArtists", "").await.ok();
    let index = &json["artists"]["index"];
    assert_eq!(names(index, "name"), ["T", "U"]);
    let artist = &index[0]["artist"][0];
    assert_eq!(artist["name"], "Test Artist");
    assert_eq!(artist["albumCount"], 1);
    let artist_id = artist["id"].as_str().unwrap().to_owned();
    assert!(artist["coverArt"].is_string());

    let json = api.call("getIndexes", "").await.ok();
    assert_eq!(
        names(&json["indexes"]["index"][1]["artist"], "name"),
        ["Unknown Artist"]
    );

    let json = api.call("getArtist", &format!("id={artist_id}")).await.ok();
    assert_eq!(names(&json["artist"]["album"], "name"), ["Test Album"]);

    let album_id = api.album_id("Test Album").await;
    let json = api.call("getAlbum", &format!("id={album_id}")).await.ok();
    let album = &json["album"];
    assert_eq!(album["songCount"], 2);
    assert_eq!(album["year"], 2024);
    assert_eq!(album["coverArt"], album_id);
    assert_eq!(
        names(&album["song"], "title"),
        ["First Light", "Second Wind"]
    );
    let song = &album["song"][1];
    assert_eq!(song["artist"], "Test Artist feat. Guest");
    assert_eq!(song["displayAlbumArtist"], "Test Artist");
    assert_eq!(song["suffix"], "mp3");
    assert_eq!(song["contentType"], "audio/mpeg");
    assert_eq!(song["duration"], 1);
    assert_eq!(song["genres"][0]["name"], "Ambient");
    let song_id = song["id"].as_str().unwrap().to_owned();

    let json = api.call("getSong", &format!("id={song_id}")).await.ok();
    assert_eq!(json["song"]["title"], "Second Wind");

    let json = api
        .call("getMusicDirectory", &format!("id={artist_id}"))
        .await
        .ok();
    assert_eq!(json["directory"]["child"][0]["isDir"], true);
    let json = api
        .call("getMusicDirectory", &format!("id={album_id}"))
        .await
        .ok();
    assert_eq!(json["directory"]["child"].as_array().unwrap().len(), 2);

    let json = api.call("getGenres", "").await.ok();
    let genre = &json["genres"]["genre"][0];
    assert_eq!(
        (&genre["value"], &genre["songCount"], &genre["albumCount"]),
        (&Value::from("Ambient"), &Value::from(2), &Value::from(1))
    );

    assert_eq!(api.call("getAlbum", "id=al-999").await.error_code(), 70);
    assert_eq!(api.call("getAlbum", "id=tr-1").await.error_code(), 70);
    assert_eq!(api.call("getAlbum", "").await.error_code(), 10);
}

#[tokio::test]
async fn album_and_song_lists() {
    let api = Api::new().await;
    let list = |kind: &str| {
        let api = &api;
        let query = format!("type={kind}&size=10");
        async move { api.call("getAlbumList2", &query).await.ok() }
    };

    let json = list("alphabeticalByName").await;
    assert_eq!(
        names(&json["albumList2"]["album"], "name"),
        ["Test Album", "Unknown Album"]
    );
    let json = list("newest").await;
    assert_eq!(json["albumList2"]["album"].as_array().unwrap().len(), 2);
    let json = list("random").await;
    assert_eq!(json["albumList2"]["album"].as_array().unwrap().len(), 2);
    let json = list("starred").await;
    assert_eq!(json["albumList2"]["album"], Value::Array(vec![]));

    let json = api
        .call("getAlbumList2", "type=byYear&fromYear=2020&toYear=2025")
        .await
        .ok();
    assert_eq!(names(&json["albumList2"]["album"], "name"), ["Test Album"]);
    let json = api
        .call("getAlbumList", "type=byGenre&genre=Ambient")
        .await
        .ok();
    assert_eq!(names(&json["albumList"]["album"], "title"), ["Test Album"]);
    assert_eq!(api.call("getAlbumList2", "").await.error_code(), 10);
    assert_eq!(
        api.call("getAlbumList2", "type=byMood").await.error_code(),
        0
    );

    let json = api.call("getRandomSongs", "size=10").await.ok();
    assert_eq!(json["randomSongs"]["song"].as_array().unwrap().len(), 3);
    let json = api.call("getSongsByGenre", "genre=Ambient").await.ok();
    assert_eq!(
        names(&json["songsByGenre"]["song"], "title"),
        ["First Light", "Second Wind"]
    );
}

#[tokio::test]
async fn searching() {
    let api = Api::new().await;

    let json = api.call("search3", "query=wind").await.ok();
    assert_eq!(
        names(&json["searchResult3"]["song"], "title"),
        ["Second Wind"]
    );

    // Every word must match somewhere: title, artist or album.
    let json = api.call("search3", "query=test%20light").await.ok();
    assert_eq!(
        names(&json["searchResult3"]["song"], "title"),
        ["First Light"]
    );
    let json = api.call("search3", "query=%22unknown%22").await.ok();
    assert_eq!(
        names(&json["searchResult3"]["artist"], "name"),
        ["Unknown Artist"]
    );

    // An empty query lists everything, for clients that sync.
    let json = api
        .call("search3", "query=&songCount=100&artistCount=0&albumCount=0")
        .await
        .ok();
    assert_eq!(json["searchResult3"]["song"].as_array().unwrap().len(), 3);
    assert_eq!(json["searchResult3"]["artist"], Value::Array(vec![]));

    let json = api.call("search2", "query=first").await.ok();
    assert_eq!(
        names(&json["searchResult2"]["song"], "title"),
        ["First Light"]
    );
}

#[tokio::test]
async fn streaming_supports_ranges() {
    let api = Api::new().await;
    let album_id = api.album_id("Test Album").await;
    let json = api.call("getAlbum", &format!("id={album_id}")).await.ok();
    let song_id = json["album"]["song"][0]["id"].as_str().unwrap().to_owned();
    let original = std::fs::read(fixture("01-first-light.flac")).unwrap();

    let full = api.call("stream", &format!("id={song_id}")).await;
    assert_eq!(full.status, StatusCode::OK);
    assert_eq!(full.headers[header::CONTENT_TYPE], "audio/flac");
    assert_eq!(full.body, original);

    let partial = api
        .request(
            Request::get(format!("/rest/stream?{AUTH}&id={song_id}"))
                .header(header::RANGE, "bytes=10-109")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(partial.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(partial.body, original[10..110]);
    assert_eq!(
        partial.headers[header::CONTENT_RANGE],
        format!("bytes 10-109/{}", original.len())
    );

    let head = api
        .request(
            Request::builder()
                .method(Method::HEAD)
                .uri(format!("/rest/stream?{AUTH}&id={song_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(head.status, StatusCode::OK);
    assert!(head.body.is_empty());

    let download = api.call("download", &format!("id={song_id}")).await;
    let disposition = download.headers[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap();
    assert!(disposition.starts_with("attachment;"), "{disposition}");
    assert!(disposition.contains("First%20Light.flac"), "{disposition}");

    assert_eq!(api.call("stream", "id=tr-999").await.error_code(), 70);
}

#[tokio::test]
async fn cover_art_is_served_and_resized() {
    let api = Api::new().await;
    let album_id = api.album_id("Test Album").await;

    let original = api.call("getCoverArt", &format!("id={album_id}")).await;
    assert_eq!(original.status, StatusCode::OK);
    assert_eq!(original.headers[header::CONTENT_TYPE], "image/png");

    let small = api
        .call("getCoverArt", &format!("id={album_id}&size=16"))
        .await;
    assert_eq!(small.headers[header::CONTENT_TYPE], "image/jpeg");
    assert!(small.body.starts_with(&[0xFF, 0xD8]), "a JPEG");

    let json = api.call("getArtists", "").await.ok();
    let artist_id = json["artists"]["index"][0]["artist"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let by_artist = api.call("getCoverArt", &format!("id={artist_id}")).await;
    assert_eq!(by_artist.status, StatusCode::OK);

    // The untagged track's album has no cover.
    let unknown = api.album_id("Unknown Album").await;
    assert_eq!(
        api.call("getCoverArt", &format!("id={unknown}"))
            .await
            .error_code(),
        70
    );
}

#[tokio::test]
async fn system_endpoints() {
    let api = Api::new().await;

    let json = api.call("getScanStatus", "").await.ok();
    assert_eq!(json["scanStatus"]["count"], 3);
    assert_eq!(json["scanStatus"]["scanning"], false);

    let json = api.call("getUser", "username=keeper").await.ok();
    assert_eq!(json["user"]["adminRole"], true);
    assert_eq!(json["user"]["folder"], serde_json::json!([1]));
    assert_eq!(
        api.call("getUser", "username=someone").await.error_code(),
        50
    );

    let unknown = api.call("getEverything", "").await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn browsers_may_call_the_api_cross_origin() {
    let api = Api::new().await;

    let preflight = api
        .request(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/rest/ping.view")
                .header(header::ORIGIN, "https://client.example")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert!(preflight.status.is_success(), "{}", preflight.status);
    assert_eq!(preflight.headers[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");

    let response = api
        .request(
            Request::get(format!("/rest/ping.view?{AUTH}"))
                .header(header::ORIGIN, "https://client.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(response.headers[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    response.ok();
}

#[tokio::test]
async fn features_to_come_answer_with_empty_lists() {
    let api = Api::new().await;

    let json = api.call("getStarred2", "").await.ok();
    assert_eq!(json["starred2"]["song"], serde_json::json!([]));
    assert_eq!(json["starred2"]["album"], serde_json::json!([]));

    let album = api.album_id("Test Album").await;
    let json = api.call("getAlbumInfo2", &format!("id={album}")).await.ok();
    assert!(json["albumInfo"].is_object());
    let json = api.call("getArtists", "").await.ok();
    let artist = json["artists"]["index"][0]["artist"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let json = api
        .call("getArtistInfo2", &format!("id={artist}"))
        .await
        .ok();
    assert_eq!(json["artistInfo2"]["similarArtist"], serde_json::json!([]));
    let json = api.call("getTopSongs", "artist=Test%20Artist").await.ok();
    assert_eq!(json["topSongs"]["song"], serde_json::json!([]));

    // Writes are not faked.
    assert_eq!(
        api.call("star", "id=tr-1").await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn play_queues_resume_across_devices() {
    let api = Api::new().await;
    let json = api.call("getPlayQueue", "").await.ok();
    assert!(json.get("playQueue").is_none(), "nothing saved yet");

    let album = api.album_id("Test Album").await;
    let json = api.call("getAlbum", &format!("id={album}")).await.ok();
    let songs = names(&json["album"]["song"], "id");

    let query = format!(
        "id={}&id={}&current={}&position=1234",
        songs[1], songs[0], songs[0]
    );
    api.call("savePlayQueue", &query).await.ok();

    let json = api.call("getPlayQueue", "").await.ok();
    let queue = &json["playQueue"];
    assert_eq!(
        names(&queue["entry"], "id"),
        [songs[1].clone(), songs[0].clone()]
    );
    assert_eq!(queue["current"], songs[0].as_str());
    assert_eq!(queue["position"], 1234);
    // The `c` parameter every request carries names the client.
    assert_eq!(queue["changedBy"], "test");

    // Saving no ids clears the queue.
    api.call("savePlayQueue", "").await.ok();
    let json = api.call("getPlayQueue", "").await.ok();
    assert!(json.get("playQueue").is_none());

    assert_eq!(api.call("savePlayQueue", "id=al-1").await.error_code(), 70);
}

#[tokio::test]
async fn scrobbles_count_plays() {
    let api = Api::new().await;
    let album = api.album_id("Test Album").await;
    let json = api.call("getAlbum", &format!("id={album}")).await.ok();
    let first = json["album"]["song"][0]["id"].as_str().unwrap().to_owned();
    assert!(json["album"].get("playCount").is_none());

    // "Now playing" notifications are not plays.
    api.call("scrobble", &format!("id={first}&submission=false"))
        .await
        .ok();
    api.call("scrobble", &format!("id={first}&time=1700000000000"))
        .await
        .ok();
    api.call("scrobble", &format!("id={first}")).await.ok();

    let json = api.call("getAlbum", &format!("id={album}")).await.ok();
    assert_eq!(json["album"]["playCount"], 2);
    assert_eq!(json["album"]["song"][0]["playCount"], 2);
    assert!(json["album"]["song"][1].get("playCount").is_none());
    assert!(json["album"]["played"].as_str().unwrap() > "2023-11-14");

    let json = api.call("getAlbumList2", "type=frequent").await.ok();
    assert_eq!(names(&json["albumList2"]["album"], "name"), ["Test Album"]);
    let json = api.call("getAlbumList2", "type=recent").await.ok();
    assert_eq!(names(&json["albumList2"]["album"], "name"), ["Test Album"]);

    assert_eq!(api.call("scrobble", "").await.error_code(), 10);
    assert_eq!(api.call("scrobble", "id=al-1").await.error_code(), 70);
}

/// Tracks that hold a claim of `kind`.
async fn claimed(db: &mut Db, kind: ClaimKind) -> Vec<u64> {
    let mut ids: Vec<u64> = TrackClaim::all()
        .exec(db)
        .await
        .unwrap()
        .into_iter()
        .filter(|claim| claim.kind == kind)
        .map(|claim| claim.track_id)
        .collect();
    ids.sort_unstable();
    ids
}

#[tokio::test]
async fn playlists_are_made_changed_and_mirrored() {
    let api = Api::new().await;
    let mut db = api.db.clone();
    let json = api.call("getPlaylists", "").await.ok();
    assert_eq!(json["playlists"]["playlist"], serde_json::json!([]));

    let album = api.album_id("Test Album").await;
    let json = api.call("getAlbum", &format!("id={album}")).await.ok();
    let songs = names(&json["album"]["song"], "id");
    let (first, second) = (&songs[0], &songs[1]);

    // Made in a client, it keeps its songs in order and claims them.
    let json = api
        .call(
            "createPlaylist",
            &format!("name=Evening&songId={second}&songId={first}"),
        )
        .await
        .ok();
    let made = &json["playlist"];
    assert_eq!(made["name"], "Evening");
    assert_eq!(made["owner"], "keeper");
    assert_eq!(made["readonly"], false);
    assert_eq!(names(&made["entry"], "id"), [second.clone(), first.clone()]);
    let id = made["id"].as_str().unwrap().to_owned();
    assert!(id.starts_with("pl-"));
    assert_eq!(claimed(&mut db, ClaimKind::LocalPlaylist).await.len(), 2);

    let json = api.call("getPlaylists", "").await.ok();
    let listed = &json["playlists"]["playlist"][0];
    assert_eq!(listed["songCount"], 2);
    assert!(listed["duration"].as_u64().unwrap() >= 2);
    assert!(listed.get("entry").is_none());
    let cover = api
        .call(
            "getCoverArt",
            &format!("id={}", listed["coverArt"].as_str().unwrap()),
        )
        .await;
    assert_eq!(cover.status, StatusCode::OK);
    assert_eq!(
        api.call("getCoverArt", &format!("id={id}")).await.status,
        StatusCode::OK
    );

    // Changed: renamed, one song out, another in.
    let untagged = api.call("search3", "query=Untitled").await.ok();
    let third = untagged["searchResult3"]["song"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    api.call(
        "updatePlaylist",
        &format!("playlistId={id}&name=Night&songIndexToRemove=0&songIdToAdd={third}"),
    )
    .await
    .ok();
    let json = api.call("getPlaylist", &format!("id={id}")).await.ok();
    assert_eq!(json["playlist"]["name"], "Night");
    assert_eq!(
        names(&json["playlist"]["entry"], "id"),
        [first.clone(), third.clone()]
    );
    assert_eq!(claimed(&mut db, ClaimKind::LocalPlaylist).await.len(), 2);

    // `createPlaylist` with an id replaces the songs.
    let json = api
        .call(
            "createPlaylist",
            &format!("playlistId={id}&songId={second}&songId={second}"),
        )
        .await
        .ok();
    assert_eq!(
        names(&json["playlist"]["entry"], "id"),
        [second.clone(), second.clone()]
    );
    assert_eq!(claimed(&mut db, ClaimKind::LocalPlaylist).await.len(), 1);
    assert_eq!(
        api.call("createPlaylist", "name=Bad&songId=tr-999")
            .await
            .error_code(),
        70
    );

    // Deleting it lets go of its songs.
    api.call("deletePlaylist", &format!("id={id}")).await.ok();
    assert_eq!(
        api.call("getPlaylist", &format!("id={id}"))
            .await
            .error_code(),
        70
    );
    assert!(claimed(&mut db, ClaimKind::LocalPlaylist).await.is_empty());

    // A mirror of a watched playlist lists what is downloaded, read-only.
    let mut track = Track::all().exec(&mut db).await.unwrap().remove(0);
    let track_id = track.id;
    toasty::update!(track {
        ytm_video_id: Some("vid-here".to_owned())
    })
    .exec(&mut db)
    .await
    .unwrap();
    let mirror = toasty::create!(Playlist {
        name: "Liked music",
        public: false,
        watch_id: Some(1_u64),
        created_at: now(),
        changed_at: now(),
    })
    .exec(&mut db)
    .await
    .unwrap();
    for (position, video) in ["vid-coming", "vid-here"].into_iter().enumerate() {
        toasty::create!(PlaylistEntry {
            playlist_id: mirror.id,
            position: position as u32,
            ytm_video_id: Some(video.to_owned()),
        })
        .exec(&mut db)
        .await
        .unwrap();
    }
    let mirror_id = format!("pl-{}", mirror.id);
    let json = api
        .call("getPlaylist", &format!("id={mirror_id}"))
        .await
        .ok();
    assert_eq!(json["playlist"]["readonly"], true);
    assert_eq!(
        names(&json["playlist"]["entry"], "id"),
        [format!("tr-{track_id}")]
    );
    for (method, query) in [
        (
            "updatePlaylist",
            format!("playlistId={mirror_id}&name=Mine"),
        ),
        ("deletePlaylist", format!("id={mirror_id}")),
        (
            "createPlaylist",
            format!("playlistId={mirror_id}&songId={first}"),
        ),
    ] {
        assert_eq!(api.call(method, &query).await.error_code(), 50, "{method}");
    }
}
