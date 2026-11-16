//! 应用目录、启动与偏好设置领域。
pub mod system_settings_apps;
pub mod aggregation;

#[cfg(test)]
mod desktop_split_13_tests;

#[cfg(test)]
mod host_module_17_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_module_25_tests;

#[cfg(not(target_arch = "wasm32"))]
#[doc(hidden)]
pub mod native_acceptance;
