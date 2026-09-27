//! Project-local migration CLI (Toasty needs our compiled models to diff them).
//!
//! After changing a model, generate a migration and commit the files under
//! `crates/pixiu-db/toasty/`:
//!
//! ```sh
//! cargo run -p pixiu-db --features cli -- migration generate --name describe_change
//! ```
//!
//! The server applies pending migrations on startup, so `migration apply` is
//! only needed against a database the server is not running on (set
//! `DATABASE_URL`, e.g. `sqlite:data/pixiu.db`).

use std::path::Path;

use toasty_cli::{Config, ToastyCli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Anchor paths to this crate so the CLI works from any directory.
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut config = Config::load_from(&crate_dir.join("Toasty.toml"))?;
    config.migration = config.migration.path(crate_dir.join("toasty"));

    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_owned());
    let db = pixiu_db::connect(&url).await?;

    ToastyCli::with_config(db, config).parse_and_run().await?;
    Ok(())
}
