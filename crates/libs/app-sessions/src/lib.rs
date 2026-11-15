//! Authorized native session catalog, resume, deletion and maintenance services.

use app_session_content::cache;
use app_session_core::source;
use app_session_codex::rollout;
#[cfg(target_os = "macos")]
use app_session_migration as migration;

mod providers {
    pub use app_session_claude::claude;
    pub use app_session_codex::codex;
    pub use app_session_gemini::gemini;
    pub use app_session_native::{omp, openclaw};
    pub use app_session_pi::pi;
    pub use app_session_sqlite::hermes;
}

pub mod catalog;
pub mod delete;
pub mod delete_batch;
pub mod identity;
pub mod resume;
pub mod terminal;
#[cfg(target_os = "macos")]
pub mod activity;
#[cfg(target_os = "macos")]
pub mod compression;
#[cfg(target_os = "macos")]
pub mod maintenance;
