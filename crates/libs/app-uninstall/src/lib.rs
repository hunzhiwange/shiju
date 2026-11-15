//! Authorized host uninstall plans and local lifecycle execution.
pub mod packages;
#[cfg(test)]
mod packages_tests;

pub mod native;
#[cfg(all(test, unix))]
mod native_tests;

pub mod execute;
#[cfg(all(test, unix))]
mod execute_tests;

pub mod local;
mod local_agy;
mod local_omp;
