//! Background work: the job queue and the session warden.

pub mod adapters;
pub mod queue;
pub mod warden;

pub use queue::{Executor, JobUpdate, Jobs, NewJob, Outcome};
pub use warden::{Health, Warden};
