pub use app_aggregation_catalog::model_ids;

pub use app_aggregation_catalog::omo_read;

#[cfg(target_os = "macos")]
pub use app_aggregation::omo_scope;
#[cfg(target_os = "macos")]
pub use app_aggregation::omo_write;

#[cfg(target_os = "macos")]
pub use app_aggregation::omo_slim;

#[cfg(target_os = "macos")]
pub use app_aggregation::failover_guard;

#[cfg(target_os = "macos")]
pub mod exit_restore;

#[cfg(all(test, target_os = "macos"))]
mod exit_restore_tests;
