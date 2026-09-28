//! Background work: the job queue and the session warden.

pub mod adapters;
pub mod enrich;
pub mod queue;
pub mod warden;
pub mod watch;

pub use queue::{Executor, Family, JobUpdate, Jobs, NewJob, Outcome, Pending, Wanted, pending};
pub use warden::{Health, Warden};
