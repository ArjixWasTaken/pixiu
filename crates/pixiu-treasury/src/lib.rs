//! The treasure: the music library píxiū owns.
//!
//! Everything that enters the hoard, whether uploaded ([`offerings`]) or
//! downloaded, goes through [`Treasury::ingest`], which files it under a
//! predictable [`layout`] and records it with a claim explaining why it is
//! kept.

pub mod covers;
mod ingest;
pub mod layout;
pub mod offerings;
pub mod tags;

pub use ingest::{Claim, IngestError, Treasury};
pub use offerings::{OfferingError, Offerings};
pub use tags::{AudioInfo, Cover, TagError};

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
