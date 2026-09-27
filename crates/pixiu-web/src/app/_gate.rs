//! Pages for signed-out visitors, framed by the logo.

mod login;
mod setup;

use topcoat::{
    Result,
    router::{Slot, layout},
    view::{View, view},
};

use crate::ui::LOGO;

#[layout]
async fn gate(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <main class="flex min-h-screen flex-col items-center justify-center gap-8 px-4 py-12">
            <div class="flex flex-col items-center gap-3 text-center">
                <img src=(LOGO) alt="" width="160" height="160" class="size-40 drop-shadow-2xl">
                <h1 class="text-4xl font-bold text-gold">"PÍXIŪ"</h1>
                <p class="text-sm text-muted-foreground">
                    "Gathers music from afar and never lets it go."
                </p>
            </div>
            <div class="w-full max-w-sm">(slot)</div>
        </main>
    })
}
