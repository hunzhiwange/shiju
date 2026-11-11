//! Chrome Native Messaging transport; website policy remains in ControlHost.
pub mod bridge;
#[cfg(test)]
mod bridge_tests;
pub mod origin_policy;
#[cfg(test)]
mod origin_policy_tests;
