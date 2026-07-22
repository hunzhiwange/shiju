//! 系统设置中模型提供商配置的目录、表单与弹窗能力。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, SETTINGS_LABEL_WIDTH,
    primary_action_btn_style, rounded_action_btn_style, settings_close_button, settings_divider,
    settings_modal_card, settings_modal_overlay, settings_muted_text_style, settings_page_intro,
    settings_panel, settings_section_card, settings_text_input_style,
};
use crate::app::{App, Message, message};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

pub(super) fn field_row<'a>(
    label: &'a str,
    description: &'a str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Start),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

pub(super) fn text_row<'a>(
    label: &'a str,
    description: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        text_input(placeholder, value)
            .on_input(on_input)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .width(Length::Fill),
    )
}

/// 构建或处理 `view_overlays` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.provider_settings;
    let mut base = dialog;

    if let Some(mm) = &s.custom_model_modal {
        let close_message = Message::Settings(message::SettingsMessage::CustomProviderModelClose);
        let close_btn = settings_close_button(close_message.clone());

        let title = if mm.edit_index.is_some() { "编辑模型" } else { "添加模型" };

        let id_row = text_row(
            "模型 ID",
            "用于请求 provider 的真实 model_id。",
            "例如 gpt-4o-mini",
            &mm.model_id,
            |v| Message::Settings(message::SettingsMessage::CustomProviderModelModalIdChanged(v)),
        );

        let name_row = text_row(
            "显示名称",
            "仅影响 UI 展示，可留空回退为 model_id。",
            "可选",
            &mm.display_name,
            |v| Message::Settings(message::SettingsMessage::CustomProviderModelModalNameChanged(v)),
        );

        let modal_col = column![
            row![
                container(settings_page_intro(title, "填写模型 ID 和展示名称。"))
                    .width(Length::Fill),
                close_btn,
            ]
            .align_y(Alignment::Start),
            settings_section_card("模型信息", "模型 ID 会直接用于请求；显示名称仅用于列表展示。",),
            settings_panel(column![id_row, settings_divider(), name_row].spacing(0)),
            row![
                container(text("")).width(Length::Fill),
                button(text("取消"))
                    .on_press(close_message.clone())
                    .padding([8, 14])
                    .style(rounded_action_btn_style),
                button(text("保存"))
                    .on_press(Message::Settings(
                        message::SettingsMessage::CustomProviderModelModalSave
                    ))
                    .padding([8, 14])
                    .style(primary_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(16);

        let card = settings_modal_card(modal_col).width(Length::Fixed(520.0));
        base = settings_modal_overlay(Some(base), close_message, card);
    }

    base
}
