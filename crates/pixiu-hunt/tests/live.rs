//! Tests against the real YouTube Music. They need network access and are
//! skipped by default: `cargo test -p pixiu-hunt -- --ignored`.
//!
//! They use a Creative Commons track (Kevin MacLeod, CC BY) so no rights are
//! at stake.

use pixiu_core::SecretBox;
use pixiu_db::Track;
use pixiu_hunt::{DownloadRequest, Hunter, YtMusic};
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
    let ytm = YtMusic::new(&dir.path().join("ytm"), SecretBox::ephemeral(), botguard()).unwrap();
    let hunter = Hunter::new(ytm, treasury.clone(), dir.path().join("staging")).unwrap();

    let found = hunter.ytmusic().search(QUERY).await.unwrap();
    let video_id = found.tracks.first().expect("a track").id.clone();

    let request = DownloadRequest {
        video_id: video_id.clone(),
        claim: Claim::offering(),
        cookies: None,
    };
    let track: Track = hunter
        .download(&request, &|percent| println!("{percent}%"))
        .await
        .unwrap();
    println!("{track:#?}");

    assert_eq!(track.ytm_video_id.as_deref(), Some(video_id.as_str()));
    let path = treasury.resolve(&track.path);
    assert!(path.is_file());
    let info = tags::read(&path).unwrap();
    assert!(info.title.unwrap().to_lowercase().contains("monkeys"));
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
