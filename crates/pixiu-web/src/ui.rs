//! Small shared components, fonts and brand assets.

use topcoat::{
    Result,
    asset::{Asset, asset},
    font::{Font, fontsource::fontsource_font},
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

pub(crate) const LOGO: Asset = asset!("assets/emblem-512.webp");
pub(crate) const LOGO_SMALL: Asset = asset!("assets/emblem-192.png");
pub(crate) const FAVICON: Asset = asset!("assets/favicon-64.png");

/// Display face for headings, echoing the logo's inscription.
pub(crate) const CINZEL: Font = fontsource_font!(
    CINZEL,
    weight: [500, 700],
    style: Normal,
    subset: Latin,
    host: Asset,
);
/// Body face.
pub(crate) const INTER: Font = fontsource_font!(
    INTER,
    weight: [400, 500, 600],
    style: Normal,
    subset: Latin,
    host: Asset,
);

pub(crate) const BUTTON_PRIMARY: StaticClass = class!(
    "inline-flex h-10 items-center justify-center gap-2 rounded-lg bg-gold px-5 \
     text-sm font-semibold text-gold-foreground shadow-sm shadow-gold/10 transition \
     hover:bg-gold-soft active:translate-y-px disabled:opacity-50",
);

pub(crate) const BUTTON_DANGER: StaticClass = class!(
    "inline-flex h-9 items-center justify-center gap-2 rounded-lg border \
     border-destructive/40 px-4 text-sm font-medium text-destructive transition \
     hover:bg-destructive/10 active:translate-y-px",
);

pub(crate) const BUTTON_GHOST: StaticClass = class!(
    "inline-flex h-9 w-full items-center gap-2 rounded-lg px-3 text-sm \
     text-muted-foreground transition hover:bg-foreground/5 hover:text-foreground",
);

const INPUT: StaticClass = class!(
    "h-10 w-full rounded-lg border border-border bg-input px-3 text-sm text-foreground \
     transition placeholder:text-muted-foreground/60 focus:border-gold/60 \
     focus:outline-none focus:ring-2 focus:ring-gold/20",
);

/// A raised panel.
#[component]
pub(crate) async fn card(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <section
            class=(class!(
                "rounded-2xl border border-border bg-card/80 p-6 shadow-xl shadow-black/30 \
                 backdrop-blur",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </section>
    })
}

/// A labelled text input. `attrs` are forwarded to the `<input>`.
#[component]
pub(crate) async fn field(label: &str, #[default] mut attrs: Attributes) -> Result<impl View> {
    Ok(view! {
        <label class="flex flex-col gap-1.5 text-sm">
            <span class="font-medium text-foreground/90">(label)</span>
            <input class=(class!(INPUT, attrs.remove("class"))) (attrs)>
        </label>
    })
}

/// A short success message.
#[component]
pub(crate) async fn notice(#[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <p
            role="status"
            class="rounded-lg border border-gold/40 bg-gold/10 px-3 py-2 text-sm text-gold-soft"
        >
            (child)
        </p>
    })
}

/// An inline error message.
#[component]
pub(crate) async fn alert(#[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <p
            role="alert"
            class="rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 \
                   text-sm text-destructive"
        >
            (child)
        </p>
    })
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
}
