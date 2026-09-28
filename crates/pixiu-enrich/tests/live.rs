//! Against the real MusicBrainz, Cover Art Archive, LRCLIB and Wikipedia:
//! `cargo test -p pixiu-enrich -- --ignored`. Kevin MacLeod's music is
//! Creative Commons.

use pixiu_enrich::{
    LocalAlbum, LocalTrack, LyricsQuery, Online, Sources,
    matching::{is_certain, pair},
};

fn track(id: u64, title: &str, seconds: u64, number: u32) -> LocalTrack {
    LocalTrack {
        id,
        title: title.to_owned(),
        artist: "Kevin MacLeod".to_owned(),
        duration_ms: seconds * 1000,
        track_number: Some(number),
        disc_number: None,
        isrc: None,
    }
}

#[tokio::test]
#[ignore = "needs network access"]
async fn an_album_is_found_and_matched() {
    let online = Online::new(None).unwrap();
    let album = LocalAlbum {
        title: "The August Album".to_owned(),
        artist: "Kevin MacLeod".to_owned(),
        year: Some(2023),
        mbid: None,
        tracks: vec![
            track(1, "Evening", 187, 1),
            track(2, "Morning", 154, 2),
            track(3, "Southern Gothic", 148, 3),
            track(4, "Vibing Over Venus", 412, 4),
        ],
    };
    let candidates = online.search(&album).await.unwrap();
    println!("{candidates:#?}");
    let best = candidates.first().expect("a candidate");
    let release = online.release(&best.id).await.unwrap();
    println!("{release:#?}");
    let pairing = pair(&album, &release);
    println!("{pairing:?}");
    assert!(is_certain(&pairing));

    let cover = online.front_cover(&release.id).await.unwrap();
    println!("cover: {:?} bytes", cover.as_ref().map(Vec::len));
    assert_eq!(cover.is_some(), release.has_front_cover);

    let artist = &release.artist.artists[0].0;
    let info = online
        .artist_info(artist)
        .await
        .unwrap()
        .expect("a biography");
    println!("{info:#?}");
    assert!(info.bio.contains("MacLeod"));
    if let Some(image) = &info.image_url {
        assert!(!online.image(image).await.unwrap().is_empty());
    }
}

#[tokio::test]
#[ignore = "needs network access"]
async fn lyrics_are_looked_up() {
    let online = Online::new(None).unwrap();
    // An instrumental, which LRCLIB knows as such (or doesn't know at all).
    let found = online
        .lyrics(&LyricsQuery {
            title: "Monkeys Spinning Monkeys".to_owned(),
            artist: "Kevin MacLeod".to_owned(),
            album: "Monkeys Spinning Monkeys".to_owned(),
            duration_secs: 125,
        })
        .await
        .unwrap();
    println!("{found:?}");
    assert!(found.is_none_or(|found| {
        found.instrumental && found.synced.is_none() && found.plain.is_none()
    }));
}
