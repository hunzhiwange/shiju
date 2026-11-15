//! Contracts shared by application and installer updates.

pub const DEFAULT_RELEASE_API: &str = "https://api.github.com/repos/hunzhiwange/shiju/releases/latest";

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateProgress {
    pub asset_name: String,
    pub download_url: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub bytes_per_second: f64,
    pub average_bytes_per_second: f64,
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
#[path = "update_tests.rs"]
mod update_tests;
