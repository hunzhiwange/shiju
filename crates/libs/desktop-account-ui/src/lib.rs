//! 账号状态、局部消息和视图。
pub mod message;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;
pub mod state;
pub mod update;
pub mod view;

pub mod account_details;

#[cfg(test)]
mod desktop_split_30_tests;
