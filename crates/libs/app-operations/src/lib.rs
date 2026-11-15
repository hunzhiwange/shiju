//! Application operations composed from their owning domain libraries.
pub use app_catalog::{acp_projection, catalog};
pub mod discovery;

pub mod lifecycle;
pub mod uninstall;
pub mod versions;

pub use app_catalog::preferences;

pub mod launch;
