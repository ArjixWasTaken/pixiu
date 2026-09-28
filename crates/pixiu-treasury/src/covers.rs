//! Resized cover art, cached on disk.

use std::{
    io,
    path::{Path, PathBuf},
};

use image::{ImageFormat, codecs::jpeg::JpegEncoder};
use sha2::{Digest, Sha256};

/// Covers are never upscaled, and requests beyond this are served as-is.
const MAX_SIZE: u32 = 2048;

/// Returns a version of `original` no larger than `size` pixels on either
/// side. Resized copies are written to `cache_dir` and reused while they are
/// newer than the original.
///
/// # Errors
///
/// Fails when the original cannot be read or decoded, or the cache cannot
/// be written.
pub async fn sized(original: &Path, cache_dir: &Path, size: Option<u32>) -> io::Result<PathBuf> {
    let Some(size) = size.filter(|size| (1..MAX_SIZE).contains(size)) else {
        return Ok(original.to_owned());
    };
    let key = cache_key(original);
    let cached = cache_dir.join("covers").join(format!("{key}-{size}.jpg"));

    if is_fresh(&cached, original).await {
        return Ok(cached);
    }

    let original = original.to_owned();
    tokio::task::spawn_blocking(move || resize(&original, &cached, size)).await?
}

/// An image's width and height, when it is one.
#[must_use]
pub fn dimensions(data: &[u8]) -> Option<(u32, u32)> {
    image::ImageReader::new(std::io::Cursor::new(data))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()
}

/// The MIME type of an image, from its content.
#[must_use]
pub fn mime_of(data: &[u8]) -> Option<&'static str> {
    Some(match image::guess_format(data).ok()? {
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::WebP => "image/webp",
        image::ImageFormat::Gif => "image/gif",
        image::ImageFormat::Bmp => "image/bmp",
        _ => return None,
    })
}

/// Removes the resized copies of `original` from the cache.
///
/// # Errors
///
/// Fails when the cache cannot be read or a copy cannot be removed.
pub async fn forget(original: &Path, cache_dir: &Path) -> io::Result<()> {
    let prefix = format!("{}-", cache_key(original));
    let mut entries = match tokio::fs::read_dir(cache_dir.join("covers")).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_name().to_string_lossy().starts_with(&prefix) {
            tokio::fs::remove_file(entry.path()).await?;
        }
    }
    Ok(())
}

fn cache_key(original: &Path) -> String {
    hex::encode(&Sha256::digest(original.as_os_str().as_encoded_bytes())[..8])
}

async fn is_fresh(cached: &Path, original: &Path) -> bool {
    let (Ok(cached), Ok(original)) = (
        tokio::fs::metadata(cached).await,
        tokio::fs::metadata(original).await,
    ) else {
        return false;
    };
    match (cached.modified(), original.modified()) {
        (Ok(cached), Ok(original)) => cached >= original,
        _ => false,
    }
}

fn resize(original: &Path, cached: &Path, size: u32) -> io::Result<PathBuf> {
    let image = image::open(original).map_err(io::Error::other)?;
    if image.width() <= size && image.height() <= size {
        return Ok(original.to_owned());
    }
    let resized = image.thumbnail(size, size).to_rgb8();

    let parent = cached.parent().expect("cache paths have a parent");
    std::fs::create_dir_all(parent)?;
    // Write to a temporary file first so concurrent readers never see a
    // partial image.
    let temp = parent.join(format!(
        ".{}.tmp",
        cached.file_name().unwrap().to_string_lossy()
    ));
    {
        let mut file = io::BufWriter::new(std::fs::File::create(&temp)?);
        resized
            .write_with_encoder(JpegEncoder::new_with_quality(&mut file, 85))
            .map_err(io::Error::other)?;
    }
    std::fs::rename(&temp, cached)?;
    Ok(cached.to_owned())
}

/// Whether `data` decodes as an image in a format browsers display.
#[must_use]
pub fn is_displayable(data: &[u8]) -> bool {
    matches!(
        image::guess_format(data),
        Ok(ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP | ImageFormat::Gif)
    )
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};

    use super::*;

    #[tokio::test]
    async fn resizes_and_caches_large_covers() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("cover.png");
        RgbImage::from_pixel(600, 400, Rgb([230, 182, 92]))
            .save(&original)
            .unwrap();
        let cache = dir.path().join("cache");

        let small = sized(&original, &cache, Some(150)).await.unwrap();
        assert_ne!(small, original);
        let image = image::open(&small).unwrap();
        assert_eq!((image.width(), image.height()), (150, 100));

        // The cached copy is reused.
        let again = sized(&original, &cache, Some(150)).await.unwrap();
        assert_eq!(again, small);

        // Never upscaled, and absent sizes serve the original.
        assert_eq!(
            sized(&original, &cache, Some(1000)).await.unwrap(),
            original
        );
        assert_eq!(sized(&original, &cache, None).await.unwrap(), original);
    }
}
