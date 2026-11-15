pub mod posix;
#[cfg(test)]
mod posix_tests;
pub use app_auth::apps::launch::request;
pub mod windows;
#[cfg(test)]
mod windows_tests;

pub mod provider;
#[cfg(test)]
mod provider_tests;

pub use app_auth::apps::launch::account;

#[cfg(unix)]
pub mod terminal_session;
#[cfg(all(test, unix))]
mod terminal_session_tests;

#[cfg(unix)]
pub mod host;
#[cfg(all(test, unix))]
mod host_tests;

#[cfg(all(test, unix))]
mod provider_omp_tests;

#[cfg(target_os = "macos")]
pub mod agy;
#[cfg(all(test, target_os = "macos"))]
mod agy_tests;
