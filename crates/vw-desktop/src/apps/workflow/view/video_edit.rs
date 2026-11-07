//! Built-in template settings. Chinese content always uses the default font.
use super::*;
use crate::app::components::system_settings_common::{
    settings_error_banner, settings_field_label, settings_group_card,
};
use iced::widget::{column, row};

pub(super) fn modal(state: &WorkflowState) -> Element<'_, Message> {
    let draft = &state.video_edit;
    let close = Message::WorkflowTool(WorkflowMessage::CloseVideoTemplate);
    let header = settings_modal_header(
        text("通用云端视频剪辑").size(20),
        settings_close_button(close.clone()),
    );
    let mut template = column![
        text("来源：随程序分发的初始模板；编辑保存在当前租户，同一模板 ID 供原生入口读取。")
            .size(13).style(settings_muted_text_style),
        text("单视频运行，最多五路。共享并发默认 5，仅允许 1..5；视频长度仍影响总耗时。")
            .size(13).style(settings_muted_text_style),
        text("YAML 中 workflow.video_edit.max_parallel 与 planning 的 max_parallel 可降低；保存影响下一次运行，运行中固定模板摘要。")
            .size(13).style(settings_muted_text_style),
    ].spacing(8);
    if let Some(record) = &draft.record {
        template = template.push(
            text(format!("已读取版本：{}", record.updated_at_ms))
                .size(12)
                .style(settings_muted_text_style),
        );
    }
    if let Some(editor) = &draft.editor {
        let mut control = text_editor(editor)
            .font(iced::Font::DEFAULT)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .padding(SETTINGS_CONTROL_PADDING)
            .style(settings_text_editor_style)
            .height(Length::Fixed(280.0));
        if !draft.busy {
            control = control
                .on_action(|a| Message::WorkflowTool(WorkflowMessage::VideoTemplateYamlAction(a)));
        }
        template = template.push(settings_field_label("完整模板 YAML")).push(control);
    } else {
        template = template.push(text("正在读取租户模板…").style(settings_muted_text_style));
    }
    let path = column![
        settings_field_label("视频路径（由原生会话校验权限，不在本机转码）"),
        row![
            workflow_text_input("选择视频路径", &draft.path, |path| {
                Message::WorkflowTool(WorkflowMessage::VideoTemplatePathChanged(path))
            }),
            action("选择视频", WorkflowMessage::PickVideoTemplatePath, true),
        ]
        .align_y(Alignment::Center)
        .spacing(8),
        text(
            "将在当前原生项目会话发送 /shiju 剪辑；使用已保存模板，未保存的 YAML 草稿不会自动应用。"
        )
        .size(13)
        .style(settings_muted_text_style),
    ]
    .spacing(8);
    let mut body = column![].spacing(12);
    if let Some(error) = &draft.error {
        body = body.push(settings_error_banner(error.as_str()));
    }
    body = body
        .push(settings_group_card(template).padding(14))
        .push(settings_group_card(path).padding(14));
    if draft.restore_confirm {
        body = body
            .push(
                text("确认恢复随程序分发的默认模板？此操作将替换服务器记录和当前 YAML 草稿。")
                    .size(14),
            )
            .push(
                row![
                    action("取消恢复", WorkflowMessage::CancelVideoTemplateRestore, !draft.busy),
                    action(
                        "确认恢复默认",
                        WorkflowMessage::ConfirmVideoTemplateRestore,
                        !draft.busy
                    )
                ]
                .spacing(10),
            );
    }
    let footer = row![
        action("关闭", WorkflowMessage::CloseVideoTemplate, true),
        action("丢弃草稿并重读", WorkflowMessage::ReloadVideoTemplate, !draft.busy),
        action("恢复默认…", WorkflowMessage::RequestVideoTemplateRestore, !draft.busy),
        action(
            "保存模板",
            WorkflowMessage::SaveVideoTemplate,
            !draft.busy && draft.record.is_some()
        ),
        button(text("在原生会话剪辑"))
            .style(primary_action_btn_style)
            .padding([9, 14])
            .on_press_maybe(
                (!draft.busy)
                    .then_some(Message::WorkflowTool(WorkflowMessage::StartNativeVideoEdit))
            ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap();
    let card = settings_modal_layout(header, body, Some(footer.into())).width(820).height(680);
    settings_modal_overlay(None, close, container(card).padding(12))
}
fn action(label: &'static str, msg: WorkflowMessage, enabled: bool) -> Element<'static, Message> {
    button(text(label))
        .style(rounded_action_btn_style)
        .padding([9, 14])
        .on_press_maybe(enabled.then_some(Message::WorkflowTool(msg)))
        .into()
}
