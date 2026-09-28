//! The treasure: the music library píxiū owns.
//!
//! Everything that enters the hoard, whether uploaded ([`offerings`]) or
//! downloaded, goes through [`Treasury::ingest`], which files it under a
//! predictable [`layout`] and records it with a claim explaining why it is
//! kept.

mod claims;
pub mod covers;
mod edit;
mod ingest;
pub mod layout;
pub mod offerings;
mod refile;
pub mod tags;

pub use claims::Release;
pub use edit::{AlbumEdit, ArtistRef, TrackEdit};
pub use ingest::{Claim, IngestError, Provenance, Treasury};
pub use layout::{Template, TemplateError};
pub use offerings::{BatchOutcome, OfferingError, Offerings};
pub use tags::{AudioInfo, Cover, TagChanges, TagError};

/// Normalizes a name for matching: case-insensitive, whitespace-collapsed.
#[must_use]
pub fn name_key(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::name_key;

    #[test]
    fn name_keys_ignore_case_and_spacing() {
        assert_eq!(name_key("  The   Beatles "), "the beatles");
        assert_eq!(name_key("ÅNGEST"), "ångest");
        assert_eq!(name_key("AC/DC"), name_key("ac/dc"));
    }
}
