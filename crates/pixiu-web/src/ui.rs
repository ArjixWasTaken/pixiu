//! Shared components, icons, fonts and brand assets.

mod components;
pub(crate) mod icons;

pub(crate) use components::*;

use jiff::Timestamp;
use pixiu_db::now;
use topcoat::{
    asset::{Asset, asset},
    font::{Font, fontsource::fontsource_font},
};

pub(crate) const LOGO: Asset = asset!("assets/emblem-512.webp");
pub(crate) const LOGO_SMALL: Asset = asset!("assets/emblem-192.png");
pub(crate) const FAVICON: Asset = asset!("assets/favicon-64.png");

/// The Material 3 face, for everything. Latin Extended carries the ū in
/// píxiū.
pub(crate) const ROBOTO: Font = fontsource_font!(
    ROBOTO,
    weight: [300, 400, 500],
    style: Normal,
    subset: [Latin, LatinExt],
    host: Asset,
);
/// For paths, keys, templates and track numbers.
pub(crate) const ROBOTO_MONO: Font = fontsource_font!(
    ROBOTO_MONO,
    weight: [400, 500],
    style: Normal,
    subset: [Latin, LatinExt],
    host: Asset,
);

/// A moment relative to now: "5 minutes ago", "in 2 hours".
pub(crate) fn relative(moment: Timestamp) -> String {
    let seconds = moment.duration_since(now()).as_secs();
    let (past, seconds) = (seconds < 0, seconds.unsigned_abs());
    let amount = |count: u64, unit: &str| {
        if count == 1 {
            format!("1 {unit}")
        } else {
            format!("{count} {unit}s")
        }
    };
    let text = match seconds {
        0..60 => return if past { "just now" } else { "in a moment" }.to_owned(),
        60..3600 => amount(seconds / 60, "minute"),
        3600..86_400 => amount(seconds / 3600, "hour"),
        _ => amount(seconds / 86_400, "day"),
    };
    if past {
        format!("{text} ago")
    } else {
        format!("in {text}")
    }
}

/// A human-readable size: `1.4 GB`.
pub(crate) fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    #[allow(clippy::cast_precision_loss)]
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// A duration as `m:ss`, or `h:mm:ss` past an hour.
pub(crate) fn format_duration(duration_ms: u64) -> String {
    let seconds = duration_ms / 1000;
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_sizes_and_durations() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1_450_000_000), "1.4 GB");
        assert_eq!(format_duration(61_000), "1:01");
        assert_eq!(format_duration(3_723_000), "1:02:03");
    }

    #[test]
    fn moments_read_relative_to_now() {
        let at = |seconds: i64| {
            now()
                .checked_add(jiff::SignedDuration::from_secs(seconds))
                .unwrap()
        };
        assert_eq!(relative(at(-5)), "just now");
        assert_eq!(relative(at(-61)), "1 minute ago");
        assert_eq!(relative(at(-7_300)), "2 hours ago");
        assert_eq!(relative(at(3 * 86_400 + 30)), "in 3 days");
    }
}
