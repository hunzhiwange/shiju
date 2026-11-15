//! 聊天交互装配；展示状态和输入面板由独立底层库提供。

#[allow(dead_code)]
pub mod chatwidget;
mod status_indicator_widget;

pub use chatwidget::{ChatWidget, ChatWidgetConfig, V3ChatStatus};
