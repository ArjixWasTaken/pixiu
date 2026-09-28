//! The design's Material 3 building blocks, as Topcoat components and class
//! helpers: buttons, text fields, switches, progress bars, chips, cards,
//! snackbars and confirmation dialogs.

use pixiu_db::SessionState;
use topcoat::{
    Result,
    context::Cx,
    icon::{IconData, icon},
    runtime::signal,
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

use super::{LOGO, icons};

/// A button's emphasis, from strongest to weakest.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Tone {
    Filled,
    Tonal,
    Outlined,
    Text,
}

/// A button's height.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Size {
    /// 32px, for dense rows.
    Xs,
    /// 40px, the usual.
    S,
    /// 56px, next to a text field.
    M,
}

/// The classes for a pill button. Works on `<a>` and `<button>` alike.
pub(crate) fn btn(tone: Tone, size: Size) -> String {
    let base = "inline-flex shrink-0 items-center justify-center gap-2 rounded-full \
                font-medium whitespace-nowrap transition select-none \
                active:translate-y-px disabled:pointer-events-none disabled:opacity-40 \
                aria-disabled:pointer-events-none aria-disabled:opacity-40";
    let size = match size {
        Size::Xs => "h-8 px-3 text-[13px]",
        Size::S => "h-10 px-4 text-sm",
        Size::M => "h-14 px-6 text-[15px]",
    };
    let tone = match tone {
        Tone::Filled => {
            "bg-gold text-gold-foreground hover:bg-gold-soft hover:text-gold-foreground"
        }
        Tone::Tonal => "bg-slate-container text-foreground hover:bg-slate hover:text-foreground",
        Tone::Outlined => "border border-outline text-gold hover:bg-gold/10 hover:text-gold",
        Tone::Text => "text-gold hover:bg-gold/10 hover:text-gold",
    };
    format!("{base} {size} {tone}")
}

/// A round, icon-only button.
pub(crate) const ICON_BUTTON: StaticClass = class!(
    "inline-grid size-10 shrink-0 place-items-center rounded-full text-muted-foreground \
     transition hover:bg-foreground/10 hover:text-foreground",
);

/// A button that looks like a link, for actions inside running text.
pub(crate) const LINK_CLASS: &str =
    "text-sm font-medium text-gold hover:text-gold-soft hover:underline";

/// A small gold, uppercase label above a heading.
pub(crate) const EYEBROW: StaticClass =
    class!("text-xs font-medium tracking-[.16em] text-gold uppercase");

/// A small slate, uppercase label inside a card.
pub(crate) const LABEL: StaticClass =
    class!("text-xs font-medium tracking-[.12em] text-slate-soft uppercase");

/// A section heading inside a page.
pub(crate) const H2: StaticClass = class!("m-0 text-[22px] font-normal");

/// A heading inside a card.
pub(crate) const CARD_TITLE: StaticClass = class!("m-0 text-xl font-normal");

/// Uppercase heading for a group of rows (Jobs: "Running", "Up next").
pub(crate) const GROUP_TITLE: StaticClass =
    class!("m-0 text-sm font-medium tracking-[.08em] text-muted-foreground uppercase");

/// One line of text that ends in an ellipsis rather than wrapping.
pub(crate) const TRUNCATE: StaticClass = class!("min-w-0 truncate");

/// The page heading: an eyebrow, the title and an optional lede, with actions
/// (the child) to the right on wide screens.
#[component]
pub(crate) async fn page_header(
    eyebrow: &str,
    title: &str,
    #[default]
    #[into]
    lede: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <header class="flex flex-wrap items-end justify-between gap-3">
            <div class="flex flex-col gap-1">
                <span class=(EYEBROW)>(eyebrow)</span>
                <h1 class="m-0 text-4xl leading-11 font-normal text-balance">(title)</h1>
                if let Some(lede) = lede {
                    <p class="m-0 max-w-[64ch] text-sm leading-5 text-pretty text-muted-foreground">
                        (lede)
                    </p>
                }
            </div>
            (child)
        </header>
    })
}

/// A filled surface card.
#[component]
pub(crate) async fn surface(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <section
            class=(class!(
                "flex flex-col gap-4 rounded-3xl bg-card p-6 [--field-bg:var(--card)]",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </section>
    })
}

/// An outlined card, for secondary panels.
#[component]
pub(crate) async fn outlined(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <section
            class=(class!(
                "flex flex-col gap-3.5 rounded-3xl border border-border p-6",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </section>
    })
}

/// A dashed, centered panel for when a list is empty.
#[component]
pub(crate) async fn empty_state(
    #[default]
    #[into]
    title: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class="flex flex-col items-center gap-2.5 rounded-3xl border border-dashed \
                   border-border px-6 py-12 text-center text-sm text-muted-foreground"
        >
            if let Some(title) = title {
                <h2 class=(class!(H2, "text-foreground"))>(title)</h2>
            }
            (child)
        </div>
    })
}

/// An error box.
#[component]
pub(crate) async fn alert(
    #[default]
    #[into]
    title: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            role="alert"
            class="flex flex-col gap-1.5 rounded-xl bg-destructive-container px-3.5 py-3 text-sm \
                   leading-5 text-destructive-soft"
        >
            if let Some(title) = title {
                <strong class="text-base font-medium text-foreground">(title)</strong>
            }
            (child)
        </div>
    })
}

/// A text field with a floating label. `attrs` go on the `<input>`.
#[component]
pub(crate) async fn text_field(
    label: &str,
    /// Filled instead of outlined.
    #[default]
    filled: bool,
    /// Help or error text under the field.
    #[default]
    #[into]
    supporting: Option<&str>,
    #[default] invalid: bool,
    #[default]
    #[into]
    leading: Option<IconData>,
    /// Shown only while the field has focus and is empty.
    #[default(" ")]
    placeholder: &str,
    #[default] mono: bool,
    #[default] mut attrs: Attributes,
) -> Result<impl View> {
    let lead = leading.is_some();
    Ok(view! {
        <label
            class=(class!(
                "flex min-w-0 flex-col gap-1",
                "field-filled" if filled,
                "field-invalid" if invalid,
                "field-lead" if lead,
                attrs.remove("wrapper-class"),
            ))
        >
            <span class="relative block">
                if let Some(leading) = leading {
                    <span
                        class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 \
                               text-muted-foreground"
                    >
                        icon(data: leading, size: 22)
                    </span>
                }
                <input
                    placeholder=(placeholder)
                    class=(class!(
                        "field-input h-14 w-full min-w-0 bg-transparent text-base text-foreground \
                         outline-none transition placeholder:opacity-0 focus:placeholder:opacity-100 \
                         placeholder:text-muted-foreground",
                        "rounded-t-sm border-b border-outline bg-muted px-4 pt-5 focus:border-b-2 \
                         focus:border-gold" if filled else "rounded-sm border border-outline px-4 \
                         hover:border-foreground-soft focus:border-2 focus:border-gold focus:px-[15px]",
                        "pl-12" if lead,
                        "font-mono text-[15px]" if mono,
                        "border-destructive focus:border-destructive" if invalid,
                    ))
                    (attrs)
                >
                <span
                    class=(class!(
                        "field-label",
                        "bg-(--field-bg,var(--background))" if !filled,
                    ))
                >
                    (label)
                </span>
            </span>
            if let Some(supporting) = supporting {
                <span
                    class=(class!(
                        "px-4 text-xs leading-4",
                        "text-destructive" if invalid else "text-muted-foreground",
                    ))
                >
                    (supporting)
                </span>
            }
        </label>
    })
}

/// A switch: a styled checkbox. `attrs` go on the `<input>`.
#[component]
pub(crate) async fn switch(
    #[default] attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <label class="inline-flex cursor-pointer items-center gap-2.5 text-sm">
            <input type="checkbox" class="peer sr-only" (attrs)>
            <span
                class="relative inline-block h-8 w-[52px] shrink-0 rounded-full border-2 \
                       border-outline bg-highest transition peer-checked:border-gold \
                       peer-checked:bg-gold peer-focus-visible:outline-2 \
                       peer-focus-visible:outline-offset-2 peer-focus-visible:outline-gold \
                       [&>span]:transition-all peer-checked:[&>span]:left-[22px] \
                       peer-checked:[&>span]:size-6 peer-checked:[&>span]:top-0.5 \
                       peer-checked:[&>span]:bg-gold-foreground"
            >
                <span class="absolute top-1.5 left-1.5 size-4 rounded-full bg-outline"></span>
            </span>
            (child)
        </label>
    })
}

/// The classes for a checkbox.
pub(crate) const CHECKBOX: StaticClass = class!("size-[18px] shrink-0 accent-gold");

/// A linear progress bar. `None` is indeterminate.
#[component]
pub(crate) async fn progress(
    value: Option<f64>,
    #[default] mut attrs: Attributes,
) -> Result<impl View> {
    let width = value.map(|value| format!("width: {:.1}%", value.clamp(0.0, 1.0) * 100.0));
    Ok(view! {
        <div
            role="progressbar"
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow=(value.map(|value| format!("{:.0}", value * 100.0)))
            class=(class!(
                "relative h-1 w-full overflow-hidden rounded-full bg-border",
                attrs.remove("class"),
            ))
            (attrs)
        >
            if let Some(width) = width {
                <div class="h-full rounded-full bg-gold transition-[width]" style=(width)></div>
            } else {
                <div class="absolute inset-y-0 w-2/5 rounded-full bg-gold animate-pxslide"></div>
            }
        </div>
    })
}

/// A filter chip, as a link.
#[component]
pub(crate) async fn chip(href: &str, selected: bool, label: &str) -> Result<impl View> {
    Ok(view! {
        <a
            href=(href)
            aria-current=(selected.then_some("page"))
            class=(class!(
                "inline-flex h-8 shrink-0 items-center gap-2 rounded-lg px-3 text-sm font-medium \
                 transition",
                "bg-slate-container text-foreground hover:text-foreground" if selected
                    else "border border-outline text-muted-foreground hover:bg-foreground/5 \
                          hover:text-foreground",
            ))
        >
            if selected {
                icon(data: icons::CHECK, size: 18)
            }
            (label)
        </a>
    })
}

/// A small rounded label: a job's state, a flag on a cover.
#[component]
pub(crate) async fn pill(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span
            class=(class!(
                "inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium \
                 whitespace-nowrap",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </span>
    })
}

/// "In the hoard": a gold ring-dot and label.
#[component]
pub(crate) async fn in_hoard() -> Result<impl View> {
    Ok(view! {
        <span class="inline-flex items-center gap-1.5 text-[13px] font-medium text-gold-soft">
            <span class="size-3.5 rounded-full bg-gold shadow-[inset_0_0_0_3px_var(--gold-container)]"></span>
            "In the hoard"
        </span>
    })
}

/// How a YouTube Music session looks.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SessionLook {
    /// For the top bar: "Connected", "Session expired".
    pub short: &'static str,
    /// For headings: "Connected", "Expired".
    pub long: &'static str,
    /// The dot's classes (color and halo).
    pub dot: &'static str,
}

pub(crate) fn session_look(state: Option<SessionState>) -> SessionLook {
    match state {
        None => SessionLook {
            short: "Not connected",
            long: "Not connected",
            dot: "bg-muted-foreground ring-muted-foreground/20",
        },
        Some(SessionState::Valid) => SessionLook {
            short: "Connected",
            long: "Connected",
            dot: "bg-success ring-success/20",
        },
        Some(SessionState::Degraded) => SessionLook {
            short: "Having trouble",
            long: "Having trouble",
            dot: "bg-gold ring-gold/20",
        },
        Some(SessionState::Expired) => SessionLook {
            short: "Session expired",
            long: "Expired",
            dot: "bg-destructive ring-destructive/20",
        },
    }
}

/// A session state's colored dot with a soft halo.
#[component]
pub(crate) async fn dot(look: SessionLook, #[default] big: bool) -> Result<impl View> {
    Ok(view! {
        <span
            class=(class!(
                "inline-block shrink-0 rounded-full",
                look.dot,
                "size-3.5 ring-[5px]" if big else "size-2.5 ring-[3px]",
            ))
        ></span>
    })
}

/// Square album art, or the emblem on a gradient when there is none.
#[component]
pub(crate) async fn cover(
    /// The album whose cover to show, if it has one.
    album: Option<u64>,
    #[default(300)] size: u32,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "relative aspect-square w-full overflow-hidden rounded-2xl bg-muted \
                 shadow-[0_1px_0_rgb(255_255_255/.04)_inset]",
                attrs.remove("class"),
            ))
            (attrs)
        >
            if let Some(album) = album {
                <img
                    src=(format!("/covers/{album}?size={size}"))
                    alt=""
                    loading="lazy"
                    class="size-full object-cover"
                >
            } else {
                <div
                    class="grid size-full place-items-center bg-[radial-gradient(circle_at_30%_25%,var(--slate),var(--background)_80%)]"
                >
                    <img src=(LOGO) alt="" class="w-1/2 opacity-25">
                </div>
            }
            (child)
        </div>
    })
}

/// A round avatar with an initial, for artists without a picture.
pub(crate) fn initial(name: &str) -> String {
    name.chars()
        .find(|c| c.is_alphanumeric())
        .map_or_else(|| "?".to_owned(), |c| c.to_uppercase().collect())
}

/// A message that slides in at the bottom of the screen and leaves on its
/// own. `action` is an optional `(label, href)`.
#[component]
pub(crate) async fn snackbar(
    cx: &Cx,
    message: &str,
    #[default]
    #[into]
    action: Option<(&str, &str)>,
    #[default] error: bool,
) -> Result<impl View> {
    let open = signal(cx, || true);
    Ok(view! {
        <div
            role=(if error { "alert" } else { "status" })
            :hidden=$(!open.get())
            class=(class!(
                "fixed bottom-24 left-1/2 z-[200] flex w-max max-w-[calc(100%-24px)] -translate-x-1/2 \
                 items-center gap-2 rounded-sm py-1 pr-1 pl-4 text-sm shadow-xl shadow-black/40 \
                 animate-pxsnack md:bottom-6 [&[hidden]]:hidden",
                "bg-destructive-container text-destructive-soft" if error
                    else "bg-foreground text-card",
            ))
        >
            <span class="py-2.5">(message)</span>
            if let Some((label, href)) = action {
                <a
                    href=(href)
                    class="rounded-full px-3 py-2 font-medium text-[#8a6420] hover:bg-black/5 \
                           hover:text-[#8a6420]"
                >
                    (label)
                </a>
            }
            <button
                type="button"
                aria-label="Dismiss"
                @click=$(|_e| open.set(false))
                class="inline-grid size-9 place-items-center rounded-full hover:bg-black/10"
            >
                icon(data: icons::CLOSE, size: 20)
            </button>
        </div>
    })
}

/// A button that copies `text` to the clipboard and says so.
#[component]
pub(crate) async fn copy_button(
    cx: &Cx,
    #[into] text: String,
    #[into]
    #[default(btn(Tone::Text, Size::S))]
    class: String,
) -> Result<impl View> {
    let copied = signal(cx, || false);
    Ok(view! {
        <button
            type="button"
            class=(class)
            // `raw!` goes last: as a statement, the runtime leaves out the
            // semicolon after it.
            @click=$(|_e| {
                copied.set(true);
                raw!("navigator.clipboard.writeText(${text})", ())
            })
        >
            $(if copied.get() { "Copied" } else { "Copy" })
        </button>
    })
}

/// A modal that asks before a destructive action. Open it with a button whose
/// `onclick` is [`open_dialog`]; without JavaScript, give the button a real
/// form action instead. The child is the explanation; `form` holds the
/// confirming form's hidden fields, if any.
#[component]
pub(crate) async fn confirm_dialog(
    id: &str,
    headline: &str,
    /// Where the confirmation posts.
    action: &str,
    confirm: &str,
    /// Another form on the page whose fields to submit, by id, instead of the
    /// dialog's own.
    #[default]
    #[into]
    submits: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <dialog
            id=(id)
            class="m-auto w-[min(560px,calc(100%-32px))] rounded-[28px] bg-muted p-6 text-foreground \
                   shadow-2xl backdrop:bg-black/50"
        >
            <form method="post" action=(action) class="flex flex-col gap-4">
                <h2 class="m-0 text-2xl font-normal">(headline)</h2>
                <div class="text-sm leading-5 text-muted-foreground">(child)</div>
                <div class="flex justify-end gap-2">
                    <button type="submit" formmethod="dialog" formnovalidate="" class=(btn(Tone::Text, Size::S))>
                        "Cancel"
                    </button>
                    <button
                        type="submit"
                        form=(submits)
                        formaction=(submits.map(|_| action))
                        class=(btn(Tone::Filled, Size::S))
                    >
                        (confirm)
                    </button>
                </div>
            </form>
        </dialog>
    })
}

/// The `onclick` script that opens the dialog `id`.
pub(crate) fn open_dialog(id: &str) -> String {
    format!("document.getElementById('{id}').showModal()")
}

/// An album in a grid: its cover, title and a line under it.
#[component]
pub(crate) async fn album_tile(
    href: &str,
    /// The album, when it has a cover.
    cover_of: Option<u64>,
    title: &str,
    #[into] subtitle: String,
    /// A small label over the cover's corner.
    #[default]
    flag: Option<&str>,
) -> Result<impl View> {
    Ok(view! {
        <a href=(href) class="group flex min-w-0 flex-col gap-2.5 text-foreground hover:text-foreground">
            cover(
                album: cover_of,
                attrs: attributes! { class="transition group-hover:opacity-90" },
                if let Some(flag) = flag {
                    <span
                        class="absolute top-2 left-2 rounded-full bg-dim/85 px-2 py-0.5 text-[11px] \
                               font-medium text-gold-soft"
                    >
                        (flag)
                    </span>
                }
            )
            <span class="flex min-w-0 flex-col gap-0.5">
                <span class=(class!(TRUNCATE, "text-[15px] font-medium"))>(title)</span>
                <span class=(class!(TRUNCATE, "text-[13px] text-muted-foreground"))>(subtitle)</span>
            </span>
        </a>
    })
}

/// The grid album tiles sit in.
pub(crate) const ALBUM_GRID: StaticClass =
    class!("grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-x-[18px] gap-y-7");

/// An artist's picture, or their initial on a gradient.
#[component]
pub(crate) async fn artist_picture(artist: &pixiu_db::Artist, size: &str) -> Result<impl View> {
    Ok(view! {
        if artist.image.is_some() {
            <img
                src=(format!("/artists/{}/image", artist.id))
                alt=""
                class=(class!("shrink-0 rounded-full border border-gold/35 object-cover", size))
            >
        } else {
            <span
                class=(class!(
                    "grid shrink-0 place-items-center rounded-full border border-gold/35 \
                     bg-[radial-gradient(circle_at_35%_30%,var(--slate-soft),var(--card)_75%)] \
                     text-foreground",
                    size,
                ))
            >
                (initial(&artist.name))
            </span>
        }
    })
}
