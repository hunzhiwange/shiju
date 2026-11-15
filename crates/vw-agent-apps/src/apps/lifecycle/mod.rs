//! Lifecycle exports and local adapter awaiting the uninstall boundary.
pub use app_lifecycle::*;
pub mod local;
pub(crate) mod local_agy;
pub(crate) mod local_omp;
