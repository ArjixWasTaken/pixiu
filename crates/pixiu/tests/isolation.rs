//! Users cannot see or change each other's libraries.
//!
//! Alice and Bob each have a library; Bob's one song is the same file as
//! one of Alice's. As Bob, every list is searched for Alice's ids, and every
//! route and Subsonic method that takes an id is called with hers: each
//! must answer "not found" (or quietly do nothing), and Alice's library
//! must stay as it was. A coverage check keeps new routes and methods from
//! slipping past the table.

use std::{collections::BTreeSet, path::Path};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use pixiu_core::{Config, SecretBox};
use pixiu_db::{
    Annotation, ApiKey, Db, Offering, Playlist, PlaylistFolder, SessionState, SourceSession, Track,
    TrackClaim, User, Watch, now, toasty,
};
use pixiu_treasury::{Claim, Provenance, tags};
use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;
use tower::ServiceExt;

const PASSWORD: &str = "gold-and-jade";

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/audio")
        .join(name)
}

struct Person {
    id: u64,
    key: String,
}

/// Alice's things, by the ids clients use.
#[derive(Default)]
struct Alices {
    tracks: Vec<u64>,
    albums: Vec<u64>,
    artists: Vec<u64>,
    orphan: u64,
    playlist: u64,
    smart_playlist: u64,
    folder: u64,
    watch: u64,
    job: u64,
    offering: u64,
    batch: String,
    key: u64,
}

impl Alices {
    /// Every Subsonic id of Alice's, which Bob must never see.
    fn subsonic_ids(&self) -> BTreeSet<String> {
        let mut ids = BTreeSet::new();
        ids.extend(self.tracks.iter().map(|id| format!("tr-{id}")));
        ids.extend(self.albums.iter().map(|id| format!("al-{id}")));
        ids.extend(self.artists.iter().map(|id| format!("ar-{id}")));
        ids.insert(format!("pl-{}", self.playlist));
        ids.insert(format!("pl-{}", self.smart_playlist));
        ids
    }
}

struct World {
    _dir: tempfile::TempDir,
    db: Db,
    services: pixiu::Services,
    router: Router,
    alice: Person,
    bob: Person,
    theirs: Alices,
}

async fn person(db: &mut Db, name: &str, secrets: &SecretBox) -> Person {
    let user = toasty::create!(User {
        username: name,
        password_hash: pixiu_core::password::hash(PASSWORD),
        subsonic_secret: Some(secrets.seal_str(PASSWORD)),
        created_at: now(),
    })
    .exec(db)
    .await
    .unwrap();
    let key = format!("pixiu_{name}_key");
    toasty::create!(ApiKey {
        user_id: user.id,
        name: "test",
        key_hash: ApiKey::hash(&key),
        created_at: now(),
    })
    .exec(db)
    .await
    .unwrap();
    Person { id: user.id, key }
}

impl World {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.paths.data_dir = dir.path().join("data");
        config.paths.treasure_dir = dir.path().join("treasure");
        let mut db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
        let secrets = SecretBox::ephemeral();
        let alice = person(&mut db, "alice", &secrets).await;
        let bob = person(&mut db, "bob", &secrets).await;
        // Alice has a YouTube Music session; Bob has none.
        toasty::create!(SourceSession {
            user_id: alice.id,
            source: "youtube_music",
            cookies: secrets.seal_str("SAPISID=alice"),
            state: SessionState::Valid,
            connected_at: now(),
        })
        .exec(&mut db)
        .await
        .unwrap();
        let services = pixiu::Services::new(db.clone(), &config, secrets)
            .await
            .unwrap();
        let web = dir.path().join("web");
        std::fs::create_dir_all(&web).unwrap();
        std::fs::write(web.join("index.html"), "<!doctype html>").unwrap();
        let router = pixiu::app(&services, &config, &web);
        let mut world = Self {
            _dir: dir,
            db,
            services,
            router,
            alice,
            bob,
            theirs: Alices::default(),
        };
        world.stock().await;
        world
    }

    async fn ingest(&self, owner: u64, name: &str) -> Track {
        let staging = self._dir.path().join("staging");
        std::fs::create_dir_all(&staging).unwrap();
        let staged = staging.join(name);
        std::fs::copy(fixture(name), &staged).unwrap();
        let info = tags::read(&staged).unwrap();
        self.services
            .treasury
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

    /// Alice's library, busy with everything a library holds; Bob's one
    /// song, the same file as one of hers.
    async fn stock(&mut self) {
        let (alice, bob) = (self.alice.id, self.bob.id);
        let mut theirs = Alices::default();
        for name in ["01-first-light.flac", "02-second-wind.mp3", "untagged.opus"] {
            let track = self.ingest(alice, name).await;
            theirs.tracks.push(track.id);
            theirs.albums.push(track.album_id);
            theirs.artists.push(track.artist_id);
        }
        let mine = self.ingest(bob, "01-first-light.flac").await;
        let alices_first = Track::get_by_id(&mut self.db, &theirs.tracks[0])
            .await
            .unwrap();
        assert_eq!(mine.file_id, alices_first.file_id, "the file is shared");

        let key = self.alice.key.clone();
        let track = |index: usize| format!("tr-{}", theirs.tracks[index]);
        self.rest(&key, "star", &format!("id={}", track(0))).await;
        self.rest(&key, "setRating", &format!("id={}&rating=4", track(0)))
            .await;
        self.rest(&key, "scrobble", &format!("id={}", track(1)))
            .await;
        self.rest(
            &key,
            "savePlayQueue",
            &format!("id={}&id={}", track(0), track(1)),
        )
        .await;
        let created = self
            .rest(
                &key,
                "createPlaylist",
                &format!("name=Mine&songId={}&songId={}", track(0), track(1)),
            )
            .await;
        theirs.playlist = number(&created["playlist"]["id"]);

        let (_, smart) = self
            .api(
                &key,
                Method::POST,
                "/api/playlists",
                Some(json!({
                    "name": "Smart",
                    "rules": [{ "id": "g", "rules": [
                        { "id": "r", "model": "title", "operator": "contains", "value": ["i"] },
                    ]}],
                })),
            )
            .await;
        theirs.smart_playlist = number(&smart["id"]);
        let (_, folder) = self
            .api(
                &key,
                Method::POST,
                "/api/playlist-folders",
                Some(json!({ "name": "Moods" })),
            )
            .await;
        theirs.folder = folder["id"].as_str().unwrap().parse().unwrap();
        let (status, _) = self
            .api(
                &key,
                Method::POST,
                "/api/watches",
                Some(json!({ "target": "https://music.youtube.com/playlist?list=PLalice12345" })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED);
        theirs.watch = Watch::filter_by_user_id(alice)
            .first()
            .exec(&mut self.db)
            .await
            .unwrap()
            .unwrap()
            .id;
        let (status, _) = self
            .api(
                &key,
                Method::POST,
                "/api/hunt/tracks",
                Some(json!({ "id": "vidalice001", "title": "Alice's grab" })),
            )
            .await;
        assert_eq!(status, StatusCode::ACCEPTED);
        theirs.job = self.services.jobs.recent(alice, 1).await.unwrap()[0].id;

        // An upload awaiting review.
        let offerings = &self.services.offerings;
        theirs.batch = pixiu_treasury::Offerings::new_batch();
        let (path, mut file) = offerings
            .create_upload(alice, &theirs.batch, "upload.mp3")
            .await
            .unwrap();
        file.write_all(&std::fs::read(fixture("02-second-wind.mp3")).unwrap())
            .await
            .unwrap();
        file.flush().await.unwrap();
        theirs.offering = offerings
            .process_upload(alice, &theirs.batch, &path)
            .await
            .unwrap()[0]
            .id;

        // An orphan: the untagged track loses its claim.
        theirs.orphan = theirs.tracks[2];
        TrackClaim::filter_by_track_id(theirs.orphan)
            .delete()
            .exec(&mut self.db)
            .await
            .unwrap();
        theirs.key = ApiKey::filter_by_user_id(alice)
            .first()
            .exec(&mut self.db)
            .await
            .unwrap()
            .unwrap()
            .id;
        self.theirs = theirs;
    }

    async fn send(&self, request: Request<Body>) -> (StatusCode, Value) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, json)
    }

    async fn api(
        &self,
        key: &str,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::AUTHORIZATION, format!("Bearer {key}"));
        let request = match body {
            Some(body) => request
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string())),
            None => request.body(Body::empty()),
        }
        .unwrap();
        self.send(request).await
    }

    /// A Subsonic call, answered as JSON (`subsonic-response`).
    async fn rest(&self, key: &str, method: &str, query: &str) -> Value {
        let uri = format!("/rest/{method}?apiKey={key}&v=1.16.1&c=test&f=json&{query}");
        let (_, json) = self
            .send(Request::get(uri).body(Body::empty()).unwrap())
            .await;
        json["subsonic-response"].clone()
    }

    /// What Alice has, to compare before and after Bob's attempts.
    async fn alices_rows(&self) -> String {
        let mut db = self.db.clone();
        let alice = self.alice.id;
        let tracks: Vec<u64> = Track::filter_by_user_id(alice)
            .exec(&mut db)
            .await
            .unwrap()
            .iter()
            .map(|track| track.id)
            .collect();
        let annotations: Vec<(String, u64, bool, Option<u8>)> =
            Annotation::filter_by_user_id(alice)
                .exec(&mut db)
                .await
                .unwrap()
                .into_iter()
                .map(|annotation| {
                    (
                        annotation.item,
                        annotation.play_count,
                        annotation.starred_at.is_some(),
                        annotation.rating,
                    )
                })
                .collect();
        let playlists: Vec<(u64, String, Option<u64>)> = Playlist::filter_by_user_id(alice)
            .exec(&mut db)
            .await
            .unwrap()
            .into_iter()
            .map(|playlist| (playlist.id, playlist.name, playlist.folder_id))
            .collect();
        let folders: Vec<(u64, String)> = PlaylistFolder::filter_by_user_id(alice)
            .exec(&mut db)
            .await
            .unwrap()
            .into_iter()
            .map(|folder| (folder.id, folder.name))
            .collect();
        let watches = Watch::filter_by_user_id(alice)
            .exec(&mut db)
            .await
            .unwrap()
            .len();
        let offerings = Offering::filter_by_user_id(alice)
            .exec(&mut db)
            .await
            .unwrap()
            .len();
        let claims = TrackClaim::all().exec(&mut db).await.unwrap().len();
        let jobs: Vec<(u64, String)> = self
            .services
            .jobs
            .recent(alice, 100)
            .await
            .unwrap()
            .into_iter()
            .map(|job| (job.id, format!("{:?}", job.state)))
            .collect();
        format!(
            "{tracks:?} {annotations:?} {playlists:?} {folders:?} {watches} {offerings} \
             {claims} {jobs:?}"
        )
    }
}

fn number(value: &Value) -> u64 {
    value
        .as_str()
        .and_then(|id| id.rsplit('-').next())
        .and_then(|id| id.parse().ok())
        .or_else(|| value.as_u64())
        .unwrap_or_else(|| panic!("not an id: {value}"))
}

/// Every string in a JSON document.
fn strings(value: &Value, into: &mut BTreeSet<String>) {
    match value {
        Value::String(text) => {
            into.insert(text.clone());
        }
        Value::Array(items) => items.iter().for_each(|item| strings(item, into)),
        Value::Object(fields) => fields.values().for_each(|field| strings(field, into)),
        _ => {}
    }
}

fn leaks(value: &Value, theirs: &BTreeSet<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    strings(value, &mut seen);
    seen.intersection(theirs).cloned().collect()
}

/// Web API routes, and whether Bob is sent Alice's ids through them. The
/// coverage test fails for a route missing here.
const API_ROUTES: &[(&str, &str)] = &[
    ("/api/auth/status", "no ids"),
    ("/api/auth/login", "no ids"),
    ("/api/auth/setup", "no ids"),
    ("/api/auth/session", "no ids"),
    ("/api/bootstrap", "list"),
    ("/api/albums", "list"),
    ("/api/artists", "list"),
    ("/api/genres", "list"),
    ("/api/songs", "list"),
    ("/api/songs/recently-played", "list"),
    ("/api/songs/{id}/info", "by id"),
    ("/api/albums/{id}/details", "by id"),
    ("/api/albums/{id}", "by id"),
    ("/api/albums/{id}/lookup", "by id"),
    ("/api/hunt", "list"),
    ("/api/hunt/tracks", "no ids"),
    ("/api/hunt/albums", "no ids"),
    ("/api/watches", "list"),
    ("/api/watches/{id}", "by id"),
    ("/api/watches/{id}/sync", "by id"),
    ("/api/watches/{id}/exclusions", "by id"),
    ("/api/watches/{id}/exclusions/{video}", "by id"),
    ("/api/playlists", "list"),
    ("/api/playlists/{id}", "by id"),
    ("/api/playlists/{id}/watch", "by id"),
    ("/api/playlist-folders", "no ids"),
    ("/api/playlist-folders/{id}", "by id"),
    ("/api/playlist-folders/{id}/playlists", "by id"),
    ("/api/hunting", "list"),
    ("/api/jobs", "list"),
    ("/api/jobs/finished", "no ids"),
    ("/api/jobs/{id}/retry", "by id"),
    ("/api/events", "list"),
    ("/api/orphans", "list"),
    ("/api/orphans/keep", "by id"),
    ("/api/orphans/delete", "by id"),
    ("/api/offerings", "list"),
    ("/api/offerings/upload", "no ids"),
    ("/api/offerings/batches/{batch}/accept", "by id"),
    ("/api/offerings/batches/{batch}", "by id"),
    ("/api/offerings/{id}", "by id"),
    ("/api/settings", "list"),
    ("/api/settings/lookup-all", "no ids"),
    ("/api/keys", "no ids"),
    ("/api/keys/{id}", "by id"),
    ("/api/sources", "list"),
    ("/api/sources/validate", "no ids"),
    ("/api/sources/refresh", "no ids"),
    ("/api/sources/disconnect", "no ids"),
    ("/api/sources/login", "no ids"),
    ("/api/sources/login/status", "no ids"),
    ("/api/sources/login/finish", "no ids"),
    ("/api/sources/login/ws", "no ids"),
];

/// Subsonic methods, and whether Bob is sent Alice's ids through them.
const SUBSONIC_METHODS: &[(&str, &str)] = &[
    ("ping", "no ids"),
    ("getLicense", "no ids"),
    ("getOpenSubsonicExtensions", "no ids"),
    ("getUser", "no ids"),
    ("getUsers", "no ids"),
    ("tokenInfo", "no ids"),
    ("getScanStatus", "list"),
    ("startScan", "list"),
    ("getMusicFolders", "no ids"),
    ("getIndexes", "list"),
    ("getArtists", "list"),
    ("getArtist", "by id"),
    ("getAlbum", "by id"),
    ("getSong", "by id"),
    ("getMusicDirectory", "by id"),
    ("getGenres", "list"),
    ("getAlbumInfo", "by id"),
    ("getAlbumInfo2", "by id"),
    ("getArtistInfo", "by id"),
    ("getArtistInfo2", "by id"),
    ("getAlbumList", "list"),
    ("getAlbumList2", "list"),
    ("getRandomSongs", "list"),
    ("getSongsByGenre", "list"),
    ("search2", "list"),
    ("search3", "list"),
    ("stream", "by id"),
    ("download", "by id"),
    ("getCoverArt", "by id"),
    ("getPlayQueue", "list"),
    ("savePlayQueue", "by id"),
    ("getLyrics", "list"),
    ("getLyricsBySongId", "by id"),
    ("getPlaylists", "list"),
    ("getPlaylist", "by id"),
    ("createPlaylist", "by id"),
    ("updatePlaylist", "by id"),
    ("deletePlaylist", "by id"),
    ("scrobble", "by id"),
    ("getNowPlaying", "list"),
    ("star", "by id"),
    ("unstar", "by id"),
    ("setRating", "by id"),
    ("getStarred", "list"),
    ("getStarred2", "list"),
    ("getTopSongs", "list"),
    ("getSimilarSongs", "by id"),
    ("getSimilarSongs2", "by id"),
];

#[test]
fn every_route_and_method_is_covered() {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../pixiu-api/src/lib.rs"),
    )
    .unwrap();
    let mut routes = BTreeSet::new();
    let mut rest = source.as_str();
    while let Some(start) = rest.find(".route(") {
        rest = &rest[start + ".route(".len()..];
        let path = rest.trim_start().trim_start_matches('"');
        let end = path.find('"').unwrap();
        routes.insert(path[..end].to_owned());
    }
    let covered: BTreeSet<String> = API_ROUTES
        .iter()
        .map(|(path, _)| (*path).to_owned())
        .collect();
    let missing: Vec<_> = routes.difference(&covered).collect();
    assert!(
        missing.is_empty(),
        "routes missing from API_ROUTES: {missing:?}"
    );
    let gone: Vec<_> = covered.difference(&routes).collect();
    assert!(
        gone.is_empty(),
        "API_ROUTES names routes that are gone: {gone:?}"
    );

    let methods: BTreeSet<&str> = pixiu_subsonic::METHODS.iter().copied().collect();
    let covered: BTreeSet<&str> = SUBSONIC_METHODS.iter().map(|(name, _)| *name).collect();
    assert_eq!(methods, covered, "SUBSONIC_METHODS must list every method");
}

#[tokio::test]
async fn lists_show_only_the_callers_library() {
    let world = World::new().await;
    let theirs = world.theirs.subsonic_ids();
    let key = world.bob.key.clone();

    for path in [
        "/api/bootstrap",
        "/api/albums",
        "/api/artists",
        "/api/songs",
        "/api/songs/recently-played",
        "/api/genres",
        "/api/playlists",
        "/api/watches",
        "/api/jobs",
        "/api/orphans",
        "/api/offerings",
        "/api/hunting",
        "/api/settings",
        "/api/sources",
    ] {
        let (status, body) = world.api(&key, Method::GET, path, None).await;
        assert_eq!(status, StatusCode::OK, "{path}: {body}");
        assert!(leaks(&body, &theirs).is_empty(), "{path} leaks {body}");
    }
    let (_, bootstrap) = world.api(&key, Method::GET, "/api/bootstrap", None).await;
    assert_eq!(bootstrap["song_count"], 1);
    assert_eq!(bootstrap["playlist_folders"], json!([]));
    for (path, empty) in [
        ("/api/watches", json!([])),
        ("/api/jobs", json!([])),
        ("/api/offerings", json!([])),
        ("/api/playlists", json!([])),
    ] {
        assert_eq!(
            world.api(&key, Method::GET, path, None).await.1,
            empty,
            "{path}"
        );
    }
    let (_, orphans) = world.api(&key, Method::GET, "/api/orphans", None).await;
    assert_eq!(orphans["orphans"], json!([]));
    let (_, hunting) = world.api(&key, Method::GET, "/api/hunting", None).await;
    assert_eq!(hunting["orphans"], 0);
    assert_eq!(hunting["offerings"], 0);
    let (_, genres) = world.api(&key, Method::GET, "/api/genres", None).await;
    assert_eq!(genres[0]["song_count"], 1, "{genres}");
    // Each has their own YouTube Music session: Bob none, Alice hers.
    let (_, sources) = world.api(&key, Method::GET, "/api/sources", None).await;
    assert_eq!(sources["health"]["state"], "none", "{sources}");
    assert_eq!(hunting["session"], "none");
    let (_, sources) = world
        .api(&world.alice.key, Method::GET, "/api/sources", None)
        .await;
    assert_eq!(sources["health"]["state"], "valid", "{sources}");

    let own_artist = Track::filter_by_user_id(world.bob.id)
        .first()
        .exec(&mut world.db.clone())
        .await
        .unwrap()
        .unwrap()
        .artist_id;
    let calls = [
        ("getArtists", String::new()),
        ("getIndexes", String::new()),
        ("getRandomSongs", "size=50".to_owned()),
        ("getSongsByGenre", "genre=Ambient&count=50".to_owned()),
        ("search3", "query=".to_owned()),
        ("search2", "query=".to_owned()),
        ("getStarred", String::new()),
        ("getStarred2", String::new()),
        ("getGenres", String::new()),
        ("getPlaylists", String::new()),
        ("getNowPlaying", String::new()),
        ("getScanStatus", String::new()),
        ("getPlayQueue", String::new()),
        ("getTopSongs", "artist=Test%20Artist".to_owned()),
        ("getSimilarSongs2", format!("id=ar-{own_artist}")),
        (
            "getLyrics",
            "artist=Test%20Artist&title=First%20Light".to_owned(),
        ),
    ]
    .into_iter()
    .chain(
        [
            "random",
            "newest",
            "alphabeticalByName",
            "alphabeticalByArtist",
            "frequent",
            "recent",
            "highest",
            "starred",
        ]
        .into_iter()
        .map(|kind| ("getAlbumList2", format!("type={kind}&size=50"))),
    )
    .chain([(
        "getAlbumList",
        "type=byGenre&genre=Ambient&size=50".to_owned(),
    )]);
    for (method, query) in calls {
        let response = world.rest(&key, method, &query).await;
        assert_eq!(response["status"], "ok", "{method}: {response}");
        assert!(
            leaks(&response, &theirs).is_empty(),
            "{method} leaks {response}"
        );
    }
    assert_eq!(
        world.rest(&key, "getScanStatus", "").await["scanStatus"]["count"],
        1
    );
    assert_eq!(
        world.rest(&key, "getStarred2", "").await["starred2"],
        json!({ "artist": [], "album": [], "song": [] })
    );
}

#[tokio::test]
async fn nobody_reaches_another_library_by_id() {
    let world = World::new().await;
    let before = world.alices_rows().await;
    let theirs = &world.theirs;
    let key = world.bob.key.clone();
    let track = format!("tr-{}", theirs.tracks[0]);
    let album = format!("al-{}", theirs.albums[0]);
    let artist = format!("ar-{}", theirs.artists[0]);
    let playlist = format!("pl-{}", theirs.playlist);
    let smart = format!("pl-{}", theirs.smart_playlist);

    // Web API: not found.
    let (_, own_folder) = world
        .api(
            &key,
            Method::POST,
            "/api/playlist-folders",
            Some(json!({ "name": "Bob's" })),
        )
        .await;
    let own_folder = own_folder["id"].as_str().unwrap().to_owned();
    let not_found = [
        (Method::GET, format!("/api/songs/{track}/info"), None),
        (Method::GET, format!("/api/albums/{album}/details"), None),
        (
            Method::PUT,
            format!("/api/albums/{album}"),
            Some(json!({ "title": "Mine now", "artist": "Bob", "year": null })),
        ),
        (Method::POST, format!("/api/albums/{album}/lookup"), None),
        (
            Method::DELETE,
            format!("/api/watches/{}", theirs.watch),
            None,
        ),
        (
            Method::POST,
            format!("/api/watches/{}/sync", theirs.watch),
            None,
        ),
        (
            Method::GET,
            format!("/api/watches/{}/exclusions", theirs.watch),
            None,
        ),
        (
            Method::POST,
            format!("/api/watches/{}/exclusions", theirs.watch),
            Some(json!({ "song": "vidalice001" })),
        ),
        (
            Method::DELETE,
            format!("/api/watches/{}/exclusions/vidalice001", theirs.watch),
            None,
        ),
        (
            Method::PUT,
            format!("/api/playlists/{smart}"),
            Some(json!({ "name": "Mine now" })),
        ),
        (
            Method::GET,
            format!("/api/playlists/{playlist}/watch"),
            None,
        ),
        (
            Method::PUT,
            format!("/api/playlist-folders/{}", theirs.folder),
            Some(json!({ "name": "Mine now" })),
        ),
        (
            Method::PATCH,
            format!("/api/playlist-folders/{}", theirs.folder),
            Some(json!({ "parent_id": null })),
        ),
        (
            Method::DELETE,
            format!("/api/playlist-folders/{}", theirs.folder),
            None,
        ),
        (
            Method::POST,
            format!("/api/playlist-folders/{}/playlists", theirs.folder),
            Some(json!({ "playlists": [] })),
        ),
        (
            Method::DELETE,
            format!("/api/playlist-folders/{}/playlists", theirs.folder),
            Some(json!({ "playlists": [] })),
        ),
        // Bob's own folder, Alice's playlist.
        (
            Method::POST,
            format!("/api/playlist-folders/{own_folder}/playlists"),
            Some(json!({ "playlists": [playlist] })),
        ),
        (
            Method::PATCH,
            format!("/api/playlist-folders/{own_folder}"),
            Some(json!({ "parent_id": theirs.folder.to_string() })),
        ),
        (
            Method::DELETE,
            format!("/api/offerings/{}", theirs.offering),
            None,
        ),
        (Method::DELETE, format!("/api/keys/{}", theirs.key), None),
    ];
    for (method, path, body) in not_found {
        let (status, response) = world.api(&key, method.clone(), &path, body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {path}: {response}");
    }

    // Web API: answered, but nothing happens to Alice's things.
    let quiet = [
        (
            Method::POST,
            format!("/api/jobs/{}/retry", theirs.job),
            None,
        ),
        (
            Method::POST,
            "/api/orphans/keep".to_owned(),
            Some(json!({ "songs": [format!("tr-{}", theirs.orphan)] })),
        ),
        (
            Method::POST,
            "/api/orphans/delete".to_owned(),
            Some(json!({ "songs": [format!("tr-{}", theirs.orphan)] })),
        ),
        (
            Method::POST,
            format!("/api/offerings/batches/{}/accept", theirs.batch),
            None,
        ),
        (
            Method::DELETE,
            format!("/api/offerings/batches/{}", theirs.batch),
            None,
        ),
    ];
    for (method, path, body) in quiet {
        let (status, response) = world.api(&key, method.clone(), &path, body).await;
        assert!(status.is_success(), "{method} {path}: {status} {response}");
    }
    let (_, kept) = world
        .api(
            &key,
            Method::POST,
            "/api/orphans/keep",
            Some(json!({ "songs": [format!("tr-{}", theirs.orphan)] })),
        )
        .await;
    assert_eq!(kept["kept"], 0);
    let (_, deleted) = world
        .api(
            &key,
            Method::POST,
            "/api/orphans/delete",
            Some(json!({ "all": true })),
        )
        .await;
    assert_eq!(deleted["deleted"], 0);

    // Subsonic: error 70, "not found".
    let not_found = [
        ("getSong", format!("id={track}")),
        ("getAlbum", format!("id={album}")),
        ("getArtist", format!("id={artist}")),
        ("getMusicDirectory", format!("id={album}")),
        ("getMusicDirectory", format!("id={artist}")),
        ("getAlbumInfo", format!("id={album}")),
        ("getAlbumInfo2", format!("id={album}")),
        ("getArtistInfo", format!("id={artist}")),
        ("getArtistInfo2", format!("id={artist}")),
        ("stream", format!("id={track}")),
        ("download", format!("id={track}")),
        ("getCoverArt", format!("id={album}")),
        ("getCoverArt", format!("id={track}")),
        ("getCoverArt", format!("id={artist}")),
        ("getCoverArt", format!("id={playlist}")),
        ("getLyricsBySongId", format!("id={track}")),
        ("getPlaylist", format!("id={playlist}")),
        ("getPlaylist", format!("id={smart}")),
        ("updatePlaylist", format!("playlistId={playlist}&name=Mine")),
        ("deletePlaylist", format!("id={playlist}")),
        ("createPlaylist", format!("playlistId={playlist}")),
        ("createPlaylist", format!("name=Stolen&songId={track}")),
        ("star", format!("id={track}")),
        ("star", format!("albumId={album}")),
        ("star", format!("artistId={artist}")),
        ("setRating", format!("id={track}&rating=1")),
        ("scrobble", format!("id={track}")),
        ("scrobble", format!("id={track}&submission=false")),
    ];
    for (method, query) in not_found {
        let response = world.rest(&key, method, &query).await;
        assert_eq!(
            response["error"]["code"], 70,
            "{method}?{query}: {response}"
        );
    }

    // Subsonic: answered, but about Bob's library only.
    let theirs_ids = theirs.subsonic_ids();
    for (method, query) in [
        ("unstar", format!("id={track}")),
        ("getSimilarSongs", format!("id={album}")),
        ("getSimilarSongs2", format!("id={track}")),
        ("savePlayQueue", format!("id={track}&current={track}")),
    ] {
        let response = world.rest(&key, method, &query).await;
        assert_eq!(response["status"], "ok", "{method}: {response}");
        assert!(
            leaks(&response, &theirs_ids).is_empty(),
            "{method}: {response}"
        );
    }
    let queue = world.rest(&key, "getPlayQueue", "").await;
    assert!(leaks(&queue, &theirs_ids).is_empty(), "{queue}");

    assert_eq!(world.alices_rows().await, before, "Alice's library changed");
    // Alice still sees her own.
    let own = world
        .rest(&world.alice.key, "getSong", &format!("id={track}"))
        .await;
    assert_eq!(own["status"], "ok", "{own}");
}

#[tokio::test]
async fn shared_files_outlive_one_library() {
    let world = World::new().await;
    let bobs = Track::filter_by_user_id(world.bob.id)
        .first()
        .exec(&mut world.db.clone())
        .await
        .unwrap()
        .unwrap();
    let file = world.services.treasury.resolve(&bobs.path);
    TrackClaim::filter_by_track_id(bobs.id)
        .delete()
        .exec(&mut world.db.clone())
        .await
        .unwrap();
    let (_, deleted) = world
        .api(
            &world.bob.key,
            Method::POST,
            "/api/orphans/delete",
            Some(json!({ "all": true })),
        )
        .await;
    assert_eq!(deleted["deleted"], 1);
    assert!(file.is_file(), "Alice still plays it");
    let response = world
        .rest(
            &world.alice.key,
            "stream",
            &format!("id=tr-{}", world.theirs.tracks[0]),
        )
        .await;
    assert!(
        response.is_null(),
        "the stream is audio, not an error: {response}"
    );
}
