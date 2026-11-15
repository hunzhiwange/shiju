pub use app_aggregation_catalog::model_ids;


pub use app_aggregation_catalog::omo_read;


pub use app_aggregation::member_keys;


pub use app_aggregation_catalog::codex_review;


pub use app_aggregation_catalog::codex_classic;


pub use app_aggregation::member_validation;


#[cfg(target_os = "macos")]
pub use app_aggregation::omo_scope;
#[cfg(target_os = "macos")]
pub use app_aggregation::omo_write;


#[cfg(target_os = "macos")]
pub use app_aggregation::omo_slim;


#[cfg(target_os = "macos")]
pub use app_aggregation::membership;


pub use app_aggregation_catalog::codex_catalog;


pub use app_aggregation_catalog::codex_subagent;


pub use app_aggregation_catalog::claude_catalog;


#[cfg(target_os = "macos")]
pub mod resolve;

#[cfg(all(test, target_os = "macos"))]
mod resolve_tests;

#[cfg(target_os = "macos")]
pub use app_aggregation::failover_guard;


#[cfg(target_os = "macos")]
pub mod preview;

#[cfg(all(test, target_os = "macos"))]
mod preview_tests;

#[cfg(target_os = "macos")]
pub mod claude_publish;

#[cfg(all(test, target_os = "macos"))]
mod claude_publish_tests;

#[cfg(target_os = "macos")]
pub mod codex_publish;

#[cfg(all(test, target_os = "macos"))]
mod codex_publish_tests;

#[cfg(all(test, target_os = "macos"))]
mod claude_refresh_tests;

#[cfg(target_os = "macos")]
pub mod catalog_generation;

#[cfg(all(test, target_os = "macos"))]
mod catalog_generation_tests;

pub use app_aggregation::codex_settings;

#[cfg(all(test, target_os = "macos"))]
mod codex_resync_tests;

#[cfg(target_os = "macos")]
pub mod exit_restore;

#[cfg(all(test, target_os = "macos"))]
mod exit_restore_tests;

#[cfg(target_os = "macos")]
pub mod stale_clients;

#[cfg(all(test, target_os = "macos"))]
mod stale_clients_tests;

#[cfg(target_os = "macos")]
pub mod catalog_ack;
#[cfg(all(test, target_os = "macos"))]
mod catalog_ack_tests;

#[cfg(all(test, target_os = "macos"))]
mod lifecycle_restore_tests;

#[cfg(target_os = "macos")]
pub mod lifecycle_restore;

#[cfg(target_os = "macos")]
pub mod daemon_restart;
