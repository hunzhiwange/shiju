//! Routes are owned by gateway-agent-apps.
pub(super) use gateway_agent_apps::launch::*;

#[cfg(all(test, unix))]
#[path = "launch_tests.rs"]
mod tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "../app_providers/agy_tests.rs"]
mod agy_tests;
