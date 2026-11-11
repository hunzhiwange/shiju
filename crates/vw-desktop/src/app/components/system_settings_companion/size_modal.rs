//! 尺寸草稿弹窗；底层页面不接受事件，取消不写入偏好。
use super::super::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button,
    settings_modal_header, settings_modal_layout, settings_modal_overlay_with_escape,
    settings_muted_text_style,
};
use crate::app::{App, Message, i18n::I18n, message::ViewMessage, state::TaskPetAvatarKind};
use iced::widget::{Space, button, column, container, row, slider, text};
use iced::{Alignment, Element, Length};

pub(crate) fn with_size_modal<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(scale) = app.task_pet_size_draft else {
        return base;
    };
    let i18n = I18n::new(app.desktop_locale);
    let pending = app.task_pet_size_pending.is_some();
    let enabled = !pending && app.task_pet_avatar_kind != TaskPetAvatarKind::None;
    let cancel = Message::View(ViewMessage::TaskPetSizeCancel);
    let header = settings_modal_header(
        text(i18n.__("自定义伙伴大小")).size(20),
        settings_close_button(cancel.clone()),
    );
    let slider =
        slider(0.5..=2.0, scale, |scale| Message::View(ViewMessage::TaskPetSizePreview(scale)))
            .step(0.05_f32);
    let slider: Element<'_, Message> = if enabled {
        slider.into()
    } else {
        // 禁用控件不发布事件，保留原生主题轨道外观。
        iced::widget::stack![
            slider,
            iced::widget::opaque(Space::new().width(Length::Fill).height(24))
        ]
        .into()
    };
    let body = column![
        text(i18n.__(if app.task_pet_avatar_kind == TaskPetAvatarKind::None {
            "当前未选择伙伴，选择伙伴后可调整大小。"
        } else {
            "拖动滑杆实时预览，工具栏与任务卡片大小保持不变。"
        }))
        .size(13)
        .style(settings_muted_text_style),
        container(super::avatar_preview(app.task_pet_avatar_kind, 74.0 * scale))
            .center_x(Length::Fill)
            .center_y(170),
        text(format!("{:.0}%", scale * 100.0)).size(16),
        slider,
    ]
    .spacing(14)
    .align_x(Alignment::Center);
    let footer = row![
        button(text(i18n.__("重置")).size(13))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe(
                (enabled && scale != 1.0).then_some(Message::View(ViewMessage::TaskPetSizeReset))
            ),
        Space::new().width(Length::Fill),
        button(text(i18n.__("取消")).size(13))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!pending).then_some(cancel.clone())),
        button(text(i18n.__(if pending { "保存中…" } else { "保存" })).size(13))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press_maybe((!pending).then_some(Message::View(ViewMessage::TaskPetSizeSave))),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fill)
        .max_width(480)
        .height(Length::Fill)
        .max_height(440);
    settings_modal_overlay_with_escape(base, cancel, container(card).padding(12))
}

#[cfg(test)]
#[path = "size_modal_tests.rs"]
mod tests;
