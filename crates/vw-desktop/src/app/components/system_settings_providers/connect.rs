//! Provider 连接模态对话框组件
//!
//! 本模块提供用于显示和管理 Provider（服务提供商）连接配置的模态对话框界面。
//! 用户可以通过此界面输入 API 密钥来连接到外部服务提供商。
//!
//! # 主要功能
//!
//! - 显示连接配置模态对话框
//! - 提供安全的 API 密钥输入界面
//! - 显示连接错误提示信息
//! - 支持通过半透明遮罩层进行交互

use crate::app::components::system_settings_common::{
    primary_action_btn_style, settings_close_button, settings_divider, settings_error_banner,
    settings_modal_card, settings_modal_overlay, settings_muted_text_style,
    settings_text_input_style,
};
use crate::app::{App, Message, message};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

/// 渲染包含连接模态对话框的覆盖层视图
///
/// 该函数负责在基础对话框之上叠加 Provider 连接模态对话框。
/// 当用户需要配置服务提供商的 API 密钥时，会显示此模态界面。
///
/// # 参数
///
/// * `app` - 应用程序状态引用，包含 Provider 设置和模态对话框状态
/// * `dialog` - 基础对话框元素，模态层将叠加在其上方
///
/// # 返回值
///
/// 返回组合后的 Element，包含原始对话框和可能的连接模态层。
/// 如果连接模态未激活，则返回原始对话框。
///
/// # UI 结构
///
/// 当模态激活时，界面由以下层级组成（从底到顶）：
/// 1. 原始对话框
/// 2. 半透明黑色遮罩层（用于视觉聚焦和点击关闭）
/// 3. 居中的连接配置卡片
///
/// # 示例
///
/// ```ignore
/// let base_dialog = text("基础界面").into();
/// let result = view_overlays(&app, base_dialog);
/// ```
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.provider_settings;
    // 基础对话框作为初始层
    let mut base = dialog;

    // 检查是否需要显示连接模态对话框
    if let Some(m) = s
        .connect_modal
        .as_ref()
        .filter(|modal| !crate::app::state::is_shiju_official_provider_id(&modal.provider_id))
    {
        let close_message = Message::Settings(message::SettingsMessage::ProviderConnectClose);
        // 创建关闭按钮（× 符号）
        let close_btn = settings_close_button(close_message.clone());

        // 构建模态对话框主体内容
        let mut modal_col = column![
            // 标题行：包含 Provider 名称、空白填充和关闭按钮
            row![
                column![
                    text(format!("配置 {} 的密钥", m.provider_name)).size(18),
                    text("录入并保存当前 Provider 的 API 密钥。")
                        .size(12)
                        .style(settings_muted_text_style),
                ]
                .spacing(4)
                .width(Length::Fill),
                container(text("")).width(Length::Fill), // 弹性填充，将关闭按钮推至右侧
                close_btn,
            ]
            .align_y(Alignment::Center),
            settings_divider(),
        ]
        .spacing(12);

        // 如果存在错误信息，在标题下方添加错误提示。
        if let Some(err) = &s.connect_error {
            modal_col = modal_col.push(settings_error_banner(err));
        }

        modal_col = modal_col.push(
            // API 密钥输入行
            row![
                text("API 密钥").width(120),
                text_input("输入 API 密钥", &m.api_key)
                    .on_input(|v| Message::Settings(
                        message::SettingsMessage::ProviderConnectApiKeyChanged(v)
                    ))
                    .width(Length::Fill)
                    .padding([10, 12])
                    .style(settings_text_input_style),
            ]
            .spacing(20)
            .align_y(Alignment::Center),
        );

        // 添加底部操作按钮行
        modal_col = modal_col.push(
            row![
                container(text("")).width(Length::Fill), // 弹性填充，将按钮推至右侧
                button(text("保存密钥"))
                    .on_press(Message::Settings(message::SettingsMessage::ProviderConnectSubmit))
                    .padding([8, 14])
                    .style(primary_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );

        // 创建模态卡片容器，应用样式（背景、边框、阴影）
        let card = settings_modal_card(modal_col).width(Length::Fixed(560.0));

        // 将所有层叠加：基础对话框 -> 遮罩层 -> 模态卡片
        base = settings_modal_overlay(Some(base), close_message, card);
    }

    base
}
