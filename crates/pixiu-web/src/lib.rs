//! The píxiū WebUI, built with Topcoat.
//!
//! Pages render on the server; routes derive from the module tree under
//! [`app`](crate::app).

mod app;
mod auth;
mod session_store;
mod ui;

pub use app::{WebDeps, router_builder};
