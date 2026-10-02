//! Writing tags into downloaded files, so every file in the treasure
//! describes itself, even outside píxiū.

use std::path::Path;

use lofty::{
    config::WriteOptions,
    file::TaggedFileExt,
    picture::{MimeType, Picture, PictureType},
    probe::Probe,
    tag::{Accessor, ItemKey, Tag, TagExt},
};
use pixiu_treasury::Cover;

use crate::HuntError;

pub(crate) struct TrackTags<'a> {
    pub title: &'a str,
    pub artist_credit: &'a str,
    pub album: &'a str,
    pub album_artist: &'a str,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u16>,
    pub isrc: Option<&'a str>,
    /// Where the track came from, written as a comment.
    pub source_url: &'a str,
    pub cover: Option<&'a Cover>,
}

/// Replaces the tags of the file at `path`. Blocking.
pub(crate) fn write(path: &Path, tags: &TrackTags<'_>) -> Result<(), HuntError> {
    let file = Probe::open(path)?.guess_file_type()?.read()?;
    let mut tag = Tag::new(file.primary_tag_type());

    tag.set_title(tags.title.to_owned());
    tag.set_artist(tags.artist_credit.to_owned());
    tag.set_album(tags.album.to_owned());
    tag.insert_text(ItemKey::AlbumArtist, tags.album_artist.to_owned());
    if let Some(number) = tags.track_number {
        tag.set_track(number);
    }
    if let Some(number) = tags.disc_number {
        tag.set_disk(number);
    }
    if let Some(year) = tags.year {
        tag.insert_text(ItemKey::RecordingDate, year.to_string());
    }
    if let Some(isrc) = tags.isrc {
        tag.insert_text(ItemKey::Isrc, isrc.to_owned());
    }
    tag.set_comment(tags.source_url.to_owned());
    if let Some(cover) = tags.cover {
        let mime = match cover.mime.as_str() {
            "image/png" => MimeType::Png,
            _ => MimeType::Jpeg,
        };
        tag.push_picture(
            Picture::unchecked(cover.data.clone())
                .pic_type(PictureType::CoverFront)
                .mime_type(mime)
                .build(),
        );
    }
    tag.save_to_path(path, WriteOptions::default())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/audio")
            .join(name)
    }

    #[test]
    fn tags_survive_a_round_trip_through_ogg_opus() {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("track.opus");
        pixiu_media::remux(&fixture("stream.webm"), &audio).unwrap();

        let cover = Cover {
            data: std::fs::read(fixture("folder.jpg")).unwrap(),
            mime: "image/jpeg".to_owned(),
        };
        write(
            &audio,
            &TrackTags {
                title: "Dawn Chorus",
                artist_credit: "Main Artist, Guest",
                album: "Morning",
                album_artist: "Main Artist",
                track_number: Some(3),
                disc_number: Some(2),
                year: Some(2021),
                isrc: Some("ZZXX12100003"),
                source_url: "https://music.youtube.com/watch?v=abc",
                cover: Some(&cover),
            },
        )
        .unwrap();

        let info = pixiu_treasury::tags::read(&audio).unwrap();
        assert_eq!(info.title.as_deref(), Some("Dawn Chorus"));
        assert_eq!(info.artist.as_deref(), Some("Main Artist, Guest"));
        assert_eq!(info.album.as_deref(), Some("Morning"));
        assert_eq!(info.album_artist.as_deref(), Some("Main Artist"));
        assert_eq!(info.track_number, Some(3));
        assert_eq!(info.disc_number, Some(2));
        assert_eq!(info.year, Some(2021));
        assert_eq!(info.isrc.as_deref(), Some("ZZXX12100003"));
        assert_eq!(info.suffix, "opus");
        assert_eq!(info.cover.map(|cover| cover.data), Some(cover.data));
    }
}
