//! Reading audio metadata.

use std::{borrow::Cow, path::Path};

use lofty::{
    file::{AudioFile, FileType, TaggedFileExt},
    picture::PictureType,
    probe::Probe,
    tag::{Accessor, ItemKey, Tag},
};

/// What píxiū knows about an audio file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioInfo {
    pub title: Option<String>,
    /// The artist credit, e.g. "Artist A feat. Artist B".
    pub artist: Option<String>,
    /// The track's artists, primary first, when the source lists them
    /// separately (streaming platforms do; file tags rarely).
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    /// MusicBrainz recording id.
    pub mbid: Option<String>,
    /// MusicBrainz release id.
    pub album_mbid: Option<String>,
    pub isrc: Option<String>,
    pub duration_ms: u64,
    /// Kilobits per second.
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub bit_depth: Option<u8>,
    /// File extension, e.g. `flac`.
    pub suffix: String,
    pub content_type: String,
    /// Embedded front cover.
    pub cover: Option<Cover>,
}

/// An image with its MIME type.
#[derive(Clone, PartialEq, Eq)]
pub struct Cover {
    pub data: Vec<u8>,
    pub mime: String,
}

impl std::fmt::Debug for Cover {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cover")
            .field("mime", &self.mime)
            .field("len", &self.data.len())
            .finish()
    }
}

impl Cover {
    /// The file extension matching the image type.
    #[must_use]
    pub fn extension(&self) -> &'static str {
        match self.mime.as_str() {
            "image/png" => "png",
            "image/webp" => "webp",
            "image/gif" => "gif",
            _ => "jpg",
        }
    }

    /// Guesses the MIME type of an image file from its extension.
    #[must_use]
    pub fn mime_for_path(path: &Path) -> Option<&'static str> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        Some(match extension.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "webp" => "image/webp",
            "gif" => "image/gif",
            _ => return None,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TagError {
    #[error("not a readable audio file: {0}")]
    Unreadable(#[from] lofty::error::FileParseError),
    #[error("not a readable audio file: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported audio format")]
    Unsupported,
}

/// Reads the tags and audio properties of the file at `path`. The format is
/// detected from the content, so misnamed files still work.
///
/// Blocking: call it from a blocking context.
///
/// # Errors
///
/// Fails when the file is not a supported audio format.
pub fn read(path: &Path) -> Result<AudioInfo, TagError> {
    let tagged = Probe::open(path)?.guess_file_type()?.read()?;
    let (suffix, content_type) = format_of(tagged.file_type()).ok_or(TagError::Unsupported)?;
    let properties = tagged.properties();

    let mut info = AudioInfo {
        duration_ms: u64::try_from(properties.duration().as_millis()).unwrap_or(u64::MAX),
        bitrate: properties
            .audio_bitrate()
            .or_else(|| properties.overall_bitrate()),
        sample_rate: properties.sample_rate(),
        channels: properties.channels(),
        bit_depth: properties.bit_depth(),
        suffix: suffix.to_owned(),
        content_type: content_type.to_owned(),
        ..AudioInfo::default()
    };

    if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
        read_tag(tag, &mut info);
    }
    Ok(info)
}

fn read_tag(tag: &Tag, info: &mut AudioInfo) {
    let text = |value: Option<Cow<'_, str>>| {
        value
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    };
    let item = |key: ItemKey| text(tag.get_string(key).map(Cow::Borrowed));

    info.title = text(tag.title());
    info.artist = text(tag.artist());
    info.album = text(tag.album());
    info.genre = text(tag.genre());
    info.album_artist = item(ItemKey::AlbumArtist);
    info.track_number = tag.track().filter(|n| *n > 0);
    info.disc_number = tag.disk().filter(|n| *n > 0);
    info.year = tag
        .date()
        .map(|date| i32::from(date.year))
        .or_else(|| item(ItemKey::Year).as_deref().and_then(leading_year))
        .or_else(|| {
            item(ItemKey::RecordingDate)
                .as_deref()
                .and_then(leading_year)
        })
        .filter(|year| *year > 0);
    info.mbid = item(ItemKey::MusicBrainzRecordingId);
    info.album_mbid = item(ItemKey::MusicBrainzReleaseId);
    info.isrc = item(ItemKey::Isrc);

    let pictures = tag.pictures();
    info.cover = pictures
        .iter()
        .find(|picture| picture.pic_type() == PictureType::CoverFront)
        .or_else(|| pictures.first())
        .map(|picture| Cover {
            data: picture.data().to_vec(),
            mime: picture
                .mime_type()
                .map_or("image/jpeg", |mime| mime.as_str())
                .to_owned(),
        });
}

/// Parses the year from dates like `2024`, `2024-03-01` or `2024/03`.
fn leading_year(date: &str) -> Option<i32> {
    date.get(..4)?.parse().ok()
}

fn format_of(file_type: FileType) -> Option<(&'static str, &'static str)> {
    Some(match file_type {
        FileType::Mpeg => ("mp3", "audio/mpeg"),
        FileType::Flac => ("flac", "audio/flac"),
        FileType::Opus => ("opus", "audio/ogg"),
        FileType::Vorbis => ("ogg", "audio/ogg"),
        FileType::Speex => ("spx", "audio/ogg"),
        FileType::Mp4 => ("m4a", "audio/mp4"),
        FileType::Aac => ("aac", "audio/aac"),
        FileType::Wav => ("wav", "audio/wav"),
        FileType::Aiff => ("aiff", "audio/aiff"),
        FileType::Ape => ("ape", "audio/x-ape"),
        FileType::WavPack => ("wv", "audio/x-wavpack"),
        FileType::Mpc => ("mpc", "audio/x-musepack"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_leading_years() {
        assert_eq!(leading_year("2024"), Some(2024));
        assert_eq!(leading_year("1999-12-31"), Some(1999));
        assert_eq!(leading_year("19"), None);
        assert_eq!(leading_year("abcd"), None);
    }
}

/// New values for a file's tags. Missing MusicBrainz ids and ISRCs leave
/// whatever the file has.
#[derive(Debug, Clone, Default)]
pub struct TagChanges<'a> {
    pub title: &'a str,
    pub artist: &'a str,
    pub album: &'a str,
    pub album_artist: &'a str,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<i32>,
    pub recording_mbid: Option<&'a str>,
    pub release_mbid: Option<&'a str>,
    pub release_group_mbid: Option<&'a str>,
    pub artist_mbid: Option<&'a str>,
    pub album_artist_mbid: Option<&'a str>,
    pub isrc: Option<&'a str>,
}

fn primary_tag(path: &Path) -> Result<Tag, TagError> {
    let file = Probe::open(path)?.guess_file_type()?.read()?;
    Ok(file
        .primary_tag()
        .cloned()
        .unwrap_or_else(|| Tag::new(file.primary_tag_type())))
}

fn save(tag: &Tag, path: &Path) -> Result<(), TagError> {
    use lofty::{config::WriteOptions, tag::TagExt};
    tag.save_to_path(path, WriteOptions::default())
        .map_err(|error| TagError::Io(std::io::Error::other(error.to_string())))
}

/// Updates the tags of the file at `path`, keeping the rest (pictures,
/// comments, ...). Blocking.
///
/// # Errors
///
/// Fails when the file cannot be read or written.
pub fn update(path: &Path, changes: &TagChanges<'_>) -> Result<(), TagError> {
    let mut tag = primary_tag(path)?;
    tag.set_title(changes.title.to_owned());
    tag.set_artist(changes.artist.to_owned());
    tag.set_album(changes.album.to_owned());
    tag.insert_text(ItemKey::AlbumArtist, changes.album_artist.to_owned());
    match changes.track_number {
        Some(number) => tag.set_track(number),
        None => tag.remove_track(),
    }
    match changes.disc_number {
        Some(number) => tag.set_disk(number),
        None => tag.remove_disk(),
    }
    if let Some(year) = changes.year {
        tag.insert_text(ItemKey::RecordingDate, year.to_string());
    }
    for (key, value) in [
        (ItemKey::MusicBrainzRecordingId, changes.recording_mbid),
        (ItemKey::MusicBrainzReleaseId, changes.release_mbid),
        (
            ItemKey::MusicBrainzReleaseGroupId,
            changes.release_group_mbid,
        ),
        (ItemKey::MusicBrainzArtistId, changes.artist_mbid),
        (
            ItemKey::MusicBrainzReleaseArtistId,
            changes.album_artist_mbid,
        ),
        (ItemKey::Isrc, changes.isrc),
    ] {
        if let Some(value) = value {
            tag.insert_text(key, value.to_owned());
        }
    }
    save(&tag, path)
}

/// Makes `cover` the file's embedded front cover. Blocking.
///
/// # Errors
///
/// Fails when the file cannot be read or written.
pub fn embed_cover(path: &Path, cover: &Cover) -> Result<(), TagError> {
    use lofty::picture::{MimeType, Picture};
    let mut tag = primary_tag(path)?;
    tag.remove_picture_type(PictureType::CoverFront);
    let mime = match cover.mime.as_str() {
        "image/png" => MimeType::Png,
        "image/gif" => MimeType::Gif,
        "image/bmp" => MimeType::Bmp,
        _ => MimeType::Jpeg,
    };
    tag.push_picture(
        Picture::unchecked(cover.data.clone())
            .pic_type(PictureType::CoverFront)
            .mime_type(mime)
            .build(),
    );
    save(&tag, path)
}

/// Lyrics embedded in the file, if any. Blocking.
#[must_use]
pub fn lyrics(path: &Path) -> Option<String> {
    let file = Probe::open(path)
        .ok()?
        .guess_file_type()
        .ok()?
        .read()
        .ok()?;
    file.tags().iter().find_map(|tag| {
        [ItemKey::Lyrics, ItemKey::UnsyncLyrics]
            .into_iter()
            .find_map(|key| tag.get_string(key))
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    })
}
