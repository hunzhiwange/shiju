//! Reviewed lifecycle recipes; execution belongs to the authorized host.
pub mod npm;
#[cfg(test)]
mod npm_tests;

pub mod native_primary;
#[cfg(test)]
mod native_primary_tests;

pub mod native_grok_mcode;
#[cfg(test)]
mod native_grok_mcode_tests;

pub mod native_hermes_agy;
#[cfg(test)]
mod native_hermes_agy_tests;

pub mod native_kimi_omp;
#[cfg(test)]
mod native_kimi_omp_tests;

pub mod update;
#[cfg(test)]
mod update_tests;

pub mod coordinator;
#[cfg(test)]
mod coordinator_tests;

pub mod process;
#[cfg(test)]
mod process_tests;

pub mod wsl;
#[cfg(test)]
mod wsl_tests;

pub mod operations;
#[cfg(test)]
mod operations_tests;
