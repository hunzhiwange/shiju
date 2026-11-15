//! Authorization account facade.
pub use app_auth::auth::*;
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) use app_auth::auth::test_support as copilot_tests;
#[cfg(all(test, target_os = "macos"))]
pub(crate) use app_auth::auth::kimi_test_support as kimi_account_tests;
