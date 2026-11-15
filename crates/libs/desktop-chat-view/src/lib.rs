//! 聊天排版的解析、窗口调度与轨迹状态。
pub mod chunk_loader;
pub mod message_view;
pub mod trajectory;
pub mod text_safety;
#[cfg(test)]
mod chunk_loader_tests;
pub mod empty;
pub mod global_status;
pub mod tool_presentation;
pub mod presentation;
#[cfg(test)]
mod global_status_tests;
pub mod render_cache;
pub mod height_index;
pub mod virtualizer;
pub mod message_measure;
#[cfg(test)]
mod height_index_tests;
#[cfg(test)]
mod virtualizer_tests;
#[cfg(test)]
mod message_measure_tests;
pub mod scroll_update;
pub mod chat;

pub mod header;
pub mod header_style;
pub mod header_usage;
#[cfg(test)]
mod header_tests;
mod header_session;

pub mod permission_anchor;
