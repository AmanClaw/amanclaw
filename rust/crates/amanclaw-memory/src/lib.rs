// sqlite module is available when "sqlite" feature is enabled (default)
#[cfg(feature = "sqlite")]
pub mod sqlite;

// TODO: pub mod postgres; (future work)

pub mod cached;
pub mod schema;
pub mod community;
pub mod vector;
