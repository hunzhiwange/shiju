//! 共享传输协议：基础响应、错误、标识符与作用域。

pub mod common;
pub mod error;
pub mod id;

pub use common::*;
pub use error::*;
pub use id::*;

#[cfg(test)]
mod common_tests;
#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod id_tests;

/// Tenant-local application settings used when no workspace project is open.
/// This context resolves to the gateway user's home directory.
pub const LOCAL_PROJECT_ID: &str = "desktop-local";

pub fn local_settings_project(project: Option<&str>) -> &str {
    project.filter(|id| !id.trim().is_empty()).unwrap_or(LOCAL_PROJECT_ID)
}

/// The gateway executes application operations on its own machine.
pub const LOCAL_HOST_ID: &str = "desktop-local";

pub fn local_project_id() -> ProjectId {
    LOCAL_PROJECT_ID.into()
}

pub fn local_host_id() -> String {
    LOCAL_HOST_ID.into()
}
