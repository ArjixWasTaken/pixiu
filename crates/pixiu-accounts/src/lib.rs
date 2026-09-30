//! Accounts: who uses píxiū, what they may do, and the rules their
//! usernames, emails and passwords follow; the server settings admins
//! change in the player; email, and the alerts it carries.

pub mod alerts;
pub mod links;
pub mod mail;
pub mod settings;
pub mod tokens;
pub mod users;

pub use mail::{Email, MailTransport, Mailer, MemoryTransport, SmtpTransport};
pub use settings::{Security, ServerSettings, Settings, Smtp};
pub use users::{AccountError, NewUser, Usage};
