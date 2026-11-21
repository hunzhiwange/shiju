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

#[cfg(unix)]
pub mod local;
#[cfg(unix)]
mod local_agy;
#[cfg(unix)]
mod local_omp;
