//! 编辑器组件、设置字段和预览标签视图。
pub mod components;

pub mod message { pub mod editor; }
pub mod metrics;
pub mod persistence;

pub mod preview;

#[cfg(not(target_arch = "wasm32"))]
pub mod status_bar;

#[cfg(test)]
mod desktop_split_01_tests;

#[cfg(test)]
mod desktop_split_08_tests;

#[cfg(test)]
mod desktop_split_22_tests;

#[cfg(test)]
mod desktop_split_27_tests;
