//! Audio work, using FFmpeg as a library.
//!
//! YouTube Music serves Opus audio in WebM. píxiū stores it as Ogg Opus
//! (`.opus`), which every player and tagger understands, by copying the
//! audio packets into a new container: no re-encoding, no quality loss.
//! For clients that want another format or a lower bitrate, [`transcode`]
//! streams a re-encoded copy; lossless downloads are stored as Opus too,
//! through [`encode`].

mod transcode;

use std::{path::Path, sync::Once, time::Duration};

pub use transcode::{Codec, Target, transcode};

use ffmpeg_next::{self as ffmpeg, codec, encoder, format, media};

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("the input has no audio stream")]
    NoAudio,
    #[error("FFmpeg was built without the {0} encoder")]
    NoEncoder(&'static str),
    #[error("FFmpeg: {0}")]
    Ffmpeg(#[from] ffmpeg::Error),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

pub(crate) fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        ffmpeg::init().expect("FFmpeg initializes");
        ffmpeg::util::log::set_level(ffmpeg::util::log::Level::Error);
    });
}

/// Copies the best audio stream of `input` into a new file at `output`,
/// whose container is chosen from its extension (`.opus` gives Ogg Opus).
///
/// Blocking: call it from a blocking context.
///
/// # Errors
///
/// Fails when the input cannot be read, has no audio, or its codec cannot be
/// stored in the output container.
pub fn remux(input: &Path, output: &Path) -> Result<(), MediaError> {
    init();
    let mut input_ctx = format::input(input)?;
    let mut output_ctx = format::output(output)?;

    let (input_index, input_time_base) = {
        let stream = input_ctx
            .streams()
            .best(media::Type::Audio)
            .ok_or(MediaError::NoAudio)?;
        let mut out_stream = output_ctx.add_stream(encoder::find(codec::Id::None))?;
        out_stream.set_parameters(stream.parameters());
        (stream.index(), stream.time_base())
    };
    output_ctx.set_metadata(input_ctx.metadata().to_owned());
    output_ctx.write_header()?;
    let output_time_base = output_ctx
        .stream(0)
        .expect("the output stream was just added")
        .time_base();

    for (stream, mut packet) in input_ctx.packets() {
        if stream.index() != input_index {
            continue;
        }
        packet.rescale_ts(input_time_base, output_time_base);
        packet.set_position(-1);
        packet.set_stream(0);
        packet.write_interleaved(&mut output_ctx)?;
    }
    output_ctx.write_trailer()?;
    Ok(())
}

/// Encodes the best audio stream of `input` into a new file at `output`,
/// in `target`'s codec and bitrate.
///
/// Blocking: call it from a blocking context.
///
/// # Errors
///
/// Fails when the output cannot be created, or as [`transcode`] does.
pub fn encode(input: &Path, output: &Path, target: Target) -> Result<(), MediaError> {
    let file = std::fs::File::create(output)?;
    transcode(input, target, Duration::ZERO, &file)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use lofty::{
        file::{AudioFile, FileType, TaggedFileExt},
        probe::Probe,
    };

    use super::*;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/audio")
            .join(name)
    }

    #[test]
    fn webm_opus_becomes_ogg_opus_without_reencoding() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("stream.opus");
        remux(&fixture("stream.webm"), &output).unwrap();

        let file = Probe::open(&output)
            .unwrap()
            .guess_file_type()
            .unwrap()
            .read()
            .unwrap();
        assert_eq!(file.file_type(), FileType::Opus);
        let millis = file.properties().duration().as_millis();
        assert!((1900..=2100).contains(&millis), "{millis} ms");
        assert_eq!(file.properties().channels(), Some(2));
        // Stream copy: the Opus payload is byte for byte the input's, so the
        // output is about the same size as the input.
        let size = std::fs::metadata(&output).unwrap().len();
        assert!((4_000..=14_000).contains(&size), "{size} bytes");
    }

    #[test]
    fn flac_is_encoded_as_ogg_opus_whole() {
        // 8 kHz, which Opus takes as is, and CD audio's 44.1 kHz, which it
        // takes as 48 kHz: every second of the song must make it.
        for name in ["01-first-light.flac", "tone-44k.flac"] {
            let dir = tempfile::tempdir().unwrap();
            let output = dir.path().join("track.opus");
            let target = Target {
                codec: Codec::Opus,
                bitrate: 160,
            };
            encode(&fixture(name), &output, target).unwrap();

            let source = Probe::open(fixture(name)).unwrap().read().unwrap();
            let file = Probe::open(&output)
                .unwrap()
                .guess_file_type()
                .unwrap()
                .read()
                .unwrap();
            assert_eq!(file.file_type(), FileType::Opus);
            let expected = source.properties().duration().as_millis();
            let millis = file.properties().duration().as_millis();
            assert!(
                expected.abs_diff(millis) <= 60,
                "{name}: {millis} ms, not {expected}"
            );
            assert_eq!(file.properties().channels(), source.properties().channels());
        }
    }

    #[test]
    fn inputs_without_audio_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let error = remux(&fixture("folder.jpg"), &dir.path().join("x.opus")).unwrap_err();
        assert!(matches!(error, MediaError::NoAudio), "{error}");
    }
}
