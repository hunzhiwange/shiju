//! 关于模态框组件
//!
//! 本模块提供应用"关于"对话框的视图实现，展示 Vibe Window 应用的基本信息。
//!
//! # 功能概述
//!
//! - 显示应用名称和描述
//! - 展示核心功能列表
//! - 显示开发者信息
//! - 提供复制应用信息和关闭按钮
//! - 半透明遮罩层背景
//!
//! # 使用示例
//!
//! ```ignore
//! use crate::app::components::about_modal;
//!
//! // 在视图层渲染关于模态框
//! let modal = about_modal::view(app);
//! ```

use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button, settings_modal_card,
    settings_modal_overlay, settings_muted_text_style,
};
use crate::app::{App, Message, message};
use iced::widget::{button, column, container, row, text, text::Wrapping};
use iced::{Alignment, Element, Length};
use std::hash::{Hash, Hasher};

const APP_NAME: &str = "VibeWindow";
const ABOUT_TITLE: &str = "关于识矩";
const ABOUT_SUBTITLE: &str =
    "Rust-first 的自主代理运行时，连接桌面端、CLI、网关服务和长驻守护进程。";
const ABOUT_DESCRIPTION: &str = "项目包含桌面应用、命令行入口、HTTP/WebSocket 网关、任务与记忆管理、通信通道、模型/提供商解析、ACP 桥接和跨平台 WebView helper。";
const AUTHOR_INFO: &str = "开发者：刘祥敏。";

fn about_copy_text() -> String {
    format!("{APP_NAME}\n{ABOUT_SUBTITLE}\n\n{ABOUT_DESCRIPTION}\n{AUTHOR_INFO}")
}

fn hash_copy_text(content: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

/// 创建关于模态框视图
///
/// 构建并返回一个完整的关于对话框组件，包含以下元素：
/// - 卡片容器：承载所有内容，带有圆角边框和阴影效果
/// - 应用信息：名称、描述和功能列表
/// - 开发者信息：显示开发者名称
/// - 复制按钮：复制当前应用信息，并在 2 秒内显示完成状态
/// - 关闭按钮：触发模态框关闭操作
/// - 遮罩层：半透明黑色背景，点击可关闭模态框
///
/// # 返回值
///
/// 返回一个 `Element<'a, Message>` 类型的 UI 元素，可被直接集成到
/// 应用的视图层次结构中。
///
/// # 交互行为
///
/// - 点击"关闭"按钮：发送 `ToggleAboutModal` 消息，关闭模态框
/// - 点击遮罩层：同样发送 `ToggleAboutModal` 消息，关闭模态框
///
/// # 示例
///
/// ```ignore
/// let about_view = view(app);
/// // 将 about_view 添加到应用的主视图中
/// ```
pub fn view<'a>(app: &'a App) -> Element<'a, Message> {
    let close_message = Message::View(message::ViewMessage::ToggleAboutModal);
    let copy_text = about_copy_text();
    let copy_label = if app.last_copied_code_hash == Some(hash_copy_text(&copy_text)) {
        "✓"
    } else {
        "复制应用信息"
    };
    let copy_button = button(text(copy_label).size(13))
        .on_press(Message::CopyCode(copy_text))
        .padding([10, 18])
        .width(Length::Fixed(132.0))
        .style(rounded_action_btn_style);

    let card = settings_modal_card(
        column![
            row![
                column![
                    text(ABOUT_TITLE).size(24),
                    text(ABOUT_SUBTITLE)
                        .size(12)
                        .width(Length::Fill)
                        .wrapping(Wrapping::Word)
                        .style(settings_muted_text_style),
                ]
                .spacing(4)
                .width(Length::Fill),
                settings_close_button(close_message.clone()),
            ]
            .align_y(Alignment::Start),
            column![
                text(ABOUT_DESCRIPTION).size(13).width(Length::Fill).wrapping(Wrapping::Word),
                text(AUTHOR_INFO)
                    .size(12)
                    .width(Length::Fill)
                    .wrapping(Wrapping::Word)
                    .style(settings_muted_text_style),
            ]
            .spacing(8),
            container(
                row![
                    copy_button,
                    button(text("关闭").size(13))
                        .on_press(close_message.clone())
                        .padding([10, 18])
                        .style(primary_action_btn_style),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(16),
    )
    .width(Length::Fixed(560.0));

    settings_modal_overlay(None, close_message, card)
}
#[cfg(test)]
#[path = "about_modal_tests.rs"]
mod about_modal_tests;
