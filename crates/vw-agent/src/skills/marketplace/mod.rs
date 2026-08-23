//! 与来源无关的技能市场提供方及安装能力入口。

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod install;
pub(crate) mod provider;
pub(crate) mod skillhub;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "install_tests.rs"]
mod install_tests;

#[cfg(test)]
#[path = "skillhub_tests.rs"]
mod skillhub_tests;
