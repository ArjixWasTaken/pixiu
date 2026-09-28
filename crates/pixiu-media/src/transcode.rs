//! Transcoding for streaming: decode, resample, encode, and write the result
//! to a pipe as it is made, so playback starts right away.

use std::{
    os::fd::{AsFd, AsRawFd},
    path::Path,
    time::Duration,
};

use ffmpeg_next::{
    self as ffmpeg, ChannelLayout, Packet, Rational, codec, encoder, format, frame, media,
    software::resampling,
};

use crate::{MediaError, init};

/// A codec píxiū can stream in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    /// MP3 (LAME): plays everywhere.
    Mp3,
    /// Opus in Ogg: the best quality for its size.
    Opus,
    /// AAC in ADTS.
    Aac,
}

impl Codec {
    /// The codec a Subsonic `format` names, if píxiū can make it.
    #[must_use]
    pub fn from_format(format: &str) -> Option<Self> {
        match format.to_ascii_lowercase().as_str() {
            "mp3" => Some(Self::Mp3),
            "opus" | "ogg" | "oga" => Some(Self::Opus),
            "aac" | "adts" | "m4a" => Some(Self::Aac),
            _ => None,
        }
    }

    /// The codec of a file with this extension, if píxiū can make it.
    #[must_use]
    pub fn from_suffix(suffix: &str) -> Option<Self> {
        match suffix.to_ascii_lowercase().as_str() {
            "mp3" => Some(Self::Mp3),
            "opus" => Some(Self::Opus),
            "aac" | "m4a" => Some(Self::Aac),
            _ => None,
        }
    }

    /// The file extension of streams in this codec.
    #[must_use]
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Opus => "opus",
            Self::Aac => "aac",
        }
    }

    #[must_use]
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mpeg",
            Self::Opus => "audio/ogg",
            Self::Aac => "audio/aac",
        }
    }

    /// Kilobits per second, when the client asks for no particular rate.
    #[must_use]
    pub fn default_bitrate(self) -> u32 {
        match self {
            Self::Mp3 => 192,
            Self::Opus => 128,
            Self::Aac => 192,
        }
    }

    /// `kbps`, within what the encoder does well.
    #[must_use]
    pub fn clamp_bitrate(self, kbps: u32) -> u32 {
        match self {
            Self::Mp3 | Self::Aac => kbps.clamp(32, 320),
            Self::Opus => kbps.clamp(8, 510),
        }
    }

    fn encoder(self) -> &'static str {
        match self {
            Self::Mp3 => "libmp3lame",
            Self::Opus => "libopus",
            Self::Aac => "aac",
        }
    }

    fn muxer(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Opus => "opus",
            Self::Aac => "adts",
        }
    }
}

/// What to transcode to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub codec: Codec,
    /// Kilobits per second.
    pub bitrate: u32,
}

/// Transcodes the audio of `input` from `offset` on, writing the stream to
/// `output` (a pipe) as it goes.
///
/// Blocking, for as long as the reader takes: run it on a thread of its own.
/// It stops with an error when the reader goes away.
///
/// # Errors
///
/// Fails when the input cannot be read or has no audio, when FFmpeg lacks
/// the encoder, or when writing fails.
pub fn transcode(
    input: &Path,
    target: Target,
    offset: Duration,
    output: &impl AsFd,
) -> Result<(), MediaError> {
    init();
    let mut input_ctx = format::input(input)?;
    let (index, input_time_base, parameters) = {
        let stream = input_ctx
            .streams()
            .best(media::Type::Audio)
            .ok_or(MediaError::NoAudio)?;
        (stream.index(), stream.time_base(), stream.parameters())
    };
    let mut decoder = codec::context::Context::from_parameters(parameters)?
        .decoder()
        .audio()?;
    if !offset.is_zero() {
        // In AV_TIME_BASE units (microseconds): the last keyframe before.
        let at = i64::try_from(offset.as_micros()).unwrap_or(i64::MAX);
        input_ctx.seek(at, ..at)?;
    }

    let codec = encoder::find_by_name(target.codec.encoder())
        .ok_or(MediaError::NoEncoder(target.codec.encoder()))?;
    let capabilities = codec.capabilities();
    let audio = codec.audio()?;
    let sample_format = pick_format(audio.formats().map(Iterator::collect).unwrap_or_default());
    let rate = pick_rate(
        audio.rates().map(Iterator::collect).unwrap_or_default(),
        decoder.rate(),
    );
    let layout = if decoder.channels() >= 2 {
        ChannelLayout::STEREO
    } else {
        ChannelLayout::MONO
    };

    // FFmpeg writes to a duplicate of the pipe, which it closes itself.
    let url = format!("pipe:{}", output.as_fd().as_raw_fd());
    let mut output_ctx = format::output_as(&url, target.codec.muxer())?;
    let mut encoder = codec::context::Context::new_with_codec(codec)
        .encoder()
        .audio()?;
    encoder.set_rate(i32::try_from(rate).unwrap_or(48_000));
    encoder.set_channel_layout(layout);
    encoder.set_format(sample_format);
    encoder.set_bit_rate(usize::try_from(target.bitrate).unwrap_or(128) * 1000);
    let encoder_time_base = Rational::new(1, i32::try_from(rate).unwrap_or(48_000));
    encoder.set_time_base(encoder_time_base);
    if output_ctx
        .format()
        .flags()
        .contains(format::Flags::GLOBAL_HEADER)
    {
        encoder.set_flags(codec::Flags::GLOBAL_HEADER);
    }
    let encoder = encoder.open_as(codec)?;
    {
        let mut stream = output_ctx.add_stream(codec)?;
        stream.set_time_base(encoder_time_base);
        stream.set_parameters(&encoder);
    }
    output_ctx.write_header()?;
    let output_time_base = output_ctx
        .stream(0)
        .expect("the output stream was just added")
        .time_base();

    let frame_size = match usize::try_from(encoder.frame_size()).unwrap_or(0) {
        0 => 4096,
        _ if capabilities.contains(codec::capabilities::Capabilities::VARIABLE_FRAME_SIZE) => 4096,
        size => size,
    };
    let mut writer = Writer {
        encoder,
        output: &mut output_ctx,
        encoder_time_base,
        output_time_base,
        fifo: Fifo::new(sample_format, layout.channels()),
        frame_size,
        layout,
        rate,
        sent: 0,
    };
    let mut resampler: Option<resampling::Context> = None;
    // Samples to drop so the stream starts at `offset`, not at the keyframe.
    let mut skip: Option<usize> = offset.is_zero().then_some(0);
    let mut decoded = frame::Audio::empty();

    let mut decode_ready = |decoder: &mut ffmpeg::decoder::Audio,
                            writer: &mut Writer<'_>,
                            resampler: &mut Option<resampling::Context>,
                            skip: &mut Option<usize>|
     -> Result<(), MediaError> {
        while decoder.receive_frame(&mut decoded).is_ok() {
            if decoded.channel_layout().is_empty() {
                decoded.set_channel_layout(ChannelLayout::default(i32::from(decoded.channels())));
            }
            let skip = skip.get_or_insert_with(|| {
                samples_before(&decoded, input_time_base, offset, writer.rate)
            });
            let resampler = match resampler {
                Some(resampler) => resampler,
                None => resampler.insert(resampling::Context::get(
                    decoded.format(),
                    decoded.channel_layout(),
                    decoded.rate(),
                    sample_format,
                    layout,
                    rate,
                )?),
            };
            let mut resampled = frame::Audio::empty();
            resampler.run(&decoded, &mut resampled)?;
            writer.fifo.push(&resampled, skip);
            writer.encode_whole_frames()?;
        }
        Ok(())
    };

    for (stream, packet) in input_ctx.packets() {
        if stream.index() != index {
            continue;
        }
        // A damaged packet is skipped, as players do.
        if decoder.send_packet(&packet).is_ok() {
            decode_ready(&mut decoder, &mut writer, &mut resampler, &mut skip)?;
        }
    }
    decoder.send_eof()?;
    decode_ready(&mut decoder, &mut writer, &mut resampler, &mut skip)?;

    if let Some(resampler) = &mut resampler {
        // What the resampler still holds (a few milliseconds at most).
        let mut tail = frame::Audio::new(sample_format, 4096, layout);
        if resampler.flush(&mut tail).is_ok() && tail.samples() > 0 {
            writer.fifo.push(&tail, &mut 0);
        }
    }
    writer.finish()?;
    output_ctx.write_trailer()?;
    Ok(())
}

/// Float if the encoder takes it, else its first choice.
fn pick_format(formats: Vec<format::Sample>) -> format::Sample {
    use format::sample::Type;
    formats
        .iter()
        .find(|format| {
            matches!(
                format,
                format::Sample::F32(Type::Packed) | format::Sample::F32(Type::Planar)
            )
        })
        .or(formats.first())
        .copied()
        .unwrap_or(format::Sample::F32(Type::Planar))
}

/// The source's rate if the encoder takes it, else the closest higher one
/// (Opus only takes 48 kHz and fractions of it), else the highest.
fn pick_rate(rates: Vec<i32>, source: u32) -> u32 {
    let rates: Vec<u32> = rates
        .into_iter()
        .filter_map(|rate| u32::try_from(rate).ok())
        .collect();
    if rates.is_empty() || rates.contains(&source) {
        return source;
    }
    rates
        .iter()
        .copied()
        .filter(|&rate| rate > source)
        .min()
        .or_else(|| rates.iter().copied().max())
        .unwrap_or(source)
}

/// How many output samples of the first frame after a seek lie before
/// `offset`: seeking lands on a keyframe, usually a little earlier.
fn samples_before(frame: &frame::Audio, time_base: Rational, offset: Duration, rate: u32) -> usize {
    let Some(pts) = frame.timestamp().or(frame.pts()) else {
        return 0;
    };
    let start = f64::from(time_base) * pts as f64;
    let early = offset.as_secs_f64() - start;
    if early <= 0.0 {
        return 0;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let samples = (early * f64::from(rate)).round() as usize;
    samples
}

/// Encodes samples in the frame sizes the encoder wants.
struct Writer<'a> {
    encoder: encoder::Audio,
    output: &'a mut format::context::Output,
    encoder_time_base: Rational,
    output_time_base: Rational,
    fifo: Fifo,
    frame_size: usize,
    layout: ChannelLayout,
    rate: u32,
    /// Samples encoded so far: the next frame's timestamp.
    sent: i64,
}

impl Writer<'_> {
    fn encode_whole_frames(&mut self) -> Result<(), MediaError> {
        while self.fifo.samples() >= self.frame_size {
            self.encode(self.frame_size)?;
        }
        Ok(())
    }

    fn encode(&mut self, samples: usize) -> Result<(), MediaError> {
        let mut frame = self.fifo.pop(samples, self.layout);
        frame.set_rate(self.rate);
        frame.set_pts(Some(self.sent));
        self.sent += i64::try_from(samples).unwrap_or(0);
        self.encoder.send_frame(&frame)?;
        self.write_packets()
    }

    fn write_packets(&mut self) -> Result<(), MediaError> {
        let mut packet = Packet::empty();
        while self.encoder.receive_packet(&mut packet).is_ok() {
            packet.set_stream(0);
            packet.rescale_ts(self.encoder_time_base, self.output_time_base);
            packet.write_interleaved(self.output)?;
        }
        Ok(())
    }

    /// Encodes what is left (the last frame may be short) and drains the
    /// encoder.
    fn finish(&mut self) -> Result<(), MediaError> {
        self.encode_whole_frames()?;
        let rest = self.fifo.samples();
        if rest > 0 {
            self.encode(rest)?;
        }
        self.encoder.send_eof()?;
        self.write_packets()
    }
}

/// Samples waiting to fill a frame: the bytes of each plane (one per
/// channel for planar formats, a single interleaved one otherwise).
struct Fifo {
    format: format::Sample,
    planes: Vec<Vec<u8>>,
    /// Bytes per sample in each plane.
    unit: usize,
}

impl Fifo {
    fn new(format: format::Sample, channels: i32) -> Self {
        let channels = usize::try_from(channels).unwrap_or(2);
        let (planes, unit) = if format.is_planar() {
            (channels, format.bytes())
        } else {
            (1, format.bytes() * channels)
        };
        Self {
            format,
            planes: vec![Vec::new(); planes],
            unit,
        }
    }

    fn samples(&self) -> usize {
        self.planes[0].len() / self.unit
    }

    /// Appends the samples of `frame`, dropping the first `skip` of them.
    fn push(&mut self, frame: &frame::Audio, skip: &mut usize) {
        let samples = frame.samples();
        let dropped = (*skip).min(samples);
        *skip -= dropped;
        for (index, plane) in self.planes.iter_mut().enumerate() {
            let data = &frame.data(index)[..samples * self.unit];
            plane.extend_from_slice(&data[dropped * self.unit..]);
        }
    }

    fn pop(&mut self, samples: usize, layout: ChannelLayout) -> frame::Audio {
        let mut frame = frame::Audio::new(self.format, samples, layout);
        let bytes = samples * self.unit;
        for (index, plane) in self.planes.iter_mut().enumerate() {
            frame.data_mut(index)[..bytes].copy_from_slice(&plane[..bytes]);
            plane.drain(..bytes);
        }
        frame
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Read, path::PathBuf};

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

    /// Transcodes through a pipe, as the server does, into a file.
    fn transcoded(input: &str, target: Target, offset: Duration) -> (tempfile::TempDir, PathBuf) {
        let (mut reader, writer) = std::io::pipe().unwrap();
        let input = fixture(input);
        let worker = std::thread::spawn(move || {
            let result = transcode(&input, target, offset, &writer);
            drop(writer);
            result
        });
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        worker.join().unwrap().unwrap();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("out.{}", target.codec.suffix()));
        std::fs::write(&path, bytes).unwrap();
        (dir, path)
    }

    fn probe(path: &Path) -> (FileType, u128) {
        let file = Probe::open(path)
            .unwrap()
            .guess_file_type()
            .unwrap()
            .read()
            .unwrap();
        (file.file_type(), file.properties().duration().as_millis())
    }

    #[test]
    fn formats_and_suffixes_name_codecs() {
        assert_eq!(Codec::from_format("MP3"), Some(Codec::Mp3));
        assert_eq!(Codec::from_format("ogg"), Some(Codec::Opus));
        assert_eq!(Codec::from_format("flac"), None);
        assert_eq!(Codec::from_suffix("m4a"), Some(Codec::Aac));
        assert_eq!(Codec::Opus.clamp_bitrate(1000), 510);
        assert_eq!(Codec::Mp3.clamp_bitrate(8), 32);
    }

    #[test]
    fn rates_suit_the_encoder() {
        // Opus: 48 kHz and fractions of it.
        let opus = vec![48_000, 24_000, 16_000, 12_000, 8_000];
        assert_eq!(pick_rate(opus.clone(), 44_100), 48_000);
        assert_eq!(pick_rate(opus.clone(), 96_000), 48_000);
        assert_eq!(pick_rate(opus, 16_000), 16_000);
        assert_eq!(pick_rate(Vec::new(), 44_100), 44_100);
    }

    #[test]
    fn flac_becomes_mp3() {
        let target = Target {
            codec: Codec::Mp3,
            bitrate: 128,
        };
        // One second of 8 kHz mono, with a cover picture as a second stream.
        let (_dir, path) = transcoded("01-first-light.flac", target, Duration::ZERO);
        let (file_type, millis) = probe(&path);
        assert_eq!(file_type, FileType::Mpeg);
        assert!((900..=1200).contains(&millis), "{millis} ms");
    }

    #[test]
    fn opus_and_aac_are_made_too() {
        for (codec, expected) in [(Codec::Opus, FileType::Opus), (Codec::Aac, FileType::Aac)] {
            let target = Target { codec, bitrate: 96 };
            let (_dir, path) = transcoded("stream.webm", target, Duration::ZERO);
            let (file_type, millis) = probe(&path);
            assert_eq!(file_type, expected);
            // ADTS has no duration header; lofty estimates it from the bitrate.
            if codec == Codec::Opus {
                assert!((1900..=2100).contains(&millis), "{millis} ms");
            }
        }
    }

    #[test]
    fn streams_can_start_later() {
        let target = Target {
            codec: Codec::Opus,
            bitrate: 96,
        };
        // Two seconds of 48 kHz stereo.
        let (_dir, path) = transcoded("stream.webm", target, Duration::from_millis(1500));
        let (_, millis) = probe(&path);
        assert!((400..=650).contains(&millis), "{millis} ms");
    }

    #[test]
    fn a_gone_reader_stops_the_transcode() {
        let (reader, writer) = std::io::pipe().unwrap();
        drop(reader);
        let target = Target {
            codec: Codec::Mp3,
            bitrate: 128,
        };
        assert!(transcode(&fixture("stream.webm"), target, Duration::ZERO, &writer).is_err());
    }
}
