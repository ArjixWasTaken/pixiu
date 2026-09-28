//! Pages for signed-out visitors, framed by the logo.

mod login;
mod setup;

use topcoat::{
    Result,
    router::{Slot, layout},
    view::{View, view},
};

/// The pages are full-bleed: each frames itself around the emblem.
#[layout]
async fn gate(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <main class="min-h-dvh bg-background">(slot)</main>
    })
}
