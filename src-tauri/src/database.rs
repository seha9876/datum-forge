//! SQLite backend module facade.
//!
//! Public IPC types and Db methods are split by responsibility under database/.
mod import_export;
mod models;
mod schema;
mod settings;
mod tables;
mod tags;
mod validation;
mod view_layout;
mod view_nav;

pub use models::*;
