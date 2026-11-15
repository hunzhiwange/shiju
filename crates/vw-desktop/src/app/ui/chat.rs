//! 聊天工具的兼容导出；实现与测试由聊天视图库拥有。

pub use desktop_chat_view::chat::{
    append_line, format_position, format_selection_positions, insert_at_cursor,
};
pub use desktop_chat_view::message_view::think::split_think;
