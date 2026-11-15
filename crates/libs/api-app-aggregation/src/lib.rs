//! Application aggregation and host-owned provider operation contracts.
mod preview;
pub use preview::*;
mod settings;
pub use settings::*;
mod directory;
pub use directory::*;
mod publication;
pub use publication::*;
pub mod operations;
#[cfg(test)]
mod aggregation_tests;
#[cfg(test)]
mod aggregation_publish_tests;
#[cfg(test)]
mod aggregation_exit_tests;
#[cfg(test)]
mod aggregation_restart_tests;
#[cfg(test)]
mod operations_tests;
