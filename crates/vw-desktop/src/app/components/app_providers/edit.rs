use super::super::system_settings_common::*;
use super::list::Help;
use crate::app::{
    Message,
    message::app_providers::edit::{EditMessage, message},
    state::app_providers::edit::{EditForm, Mode},
};
use iced::{
    Alignment, Element, Length,
    widget::{button, checkbox, column, container, row, text, text_input},
};
pub(crate) fn overlay<'a>(
    form: &'a EditForm,
    help: Help<'_>,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let title = match form.mode {
        Mode::Create => "新增供应商",
        Mode::Update => "编辑供应商",
        Mode::Copy { .. } => "复制供应商",
        Mode::Delete => "删除供应商",
    };
    let input = |label: &'static str, value: &'a str, event: fn(String) -> EditMessage| {
        text_input(label, value)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe((!form.saving).then_some(move |v| message(event(v))))
    };
    let mut fields = column![].spacing(12).width(Length::Fill);
    let delete = matches!(form.mode, Mode::Delete);
    if delete {
        fields = fields.push(settings_panel(text(format!(
            "确认删除「{}」？执行端会检查当前选择与引用，失败时保留此选择。",
            form.fields.name
        ))));
    } else {
        let mut basic = column![].spacing(8);
        if matches!(form.mode, Mode::Create | Mode::Copy { .. }) {
            basic = basic.push(settings_field_label("新标识")).push(settings_control_with_hint(
                input("新标识", &form.id, EditMessage::Id),
                "ASCII 字母、数字、短横线或下划线",
            ));
        }
        basic = basic.push(settings_field_label("名称")).push(input(
            "名称",
            &form.fields.name,
            EditMessage::Name,
        ));
        if !form.claude_editable() {
            basic = basic.push(settings_field_label("分类")).push(input(
                "分类",
                form.fields.category.as_deref().unwrap_or(""),
                EditMessage::Category,
            ));
        }
        basic = basic
            .push(settings_field_label("备注"))
            .push(input("备注", form.fields.notes.as_deref().unwrap_or(""), EditMessage::Notes))
            .push(help.field("网址", "展示网址，与 API 地址分开保存"))
            .push(input(
                "https://…",
                form.fields.website_url.as_deref().unwrap_or(""),
                EditMessage::Website,
            ));
        fields = fields.push(text("基础资料").size(17)).push(settings_panel(basic));
        if let Some(draft) = &form.claude {
            fields = fields.push(super::claude::fields(form, draft, help));
        } else if let Some(draft) = &form.codex {
            fields = fields.push(super::codex::fields(form, draft, help));
        } else if let Some(draft) = &form.gemini {
            fields = fields.push(super::gemini::fields(form, draft, help));
        } else if let Some(draft) = &form.grok {
            fields = fields.push(super::grok::fields(form, draft, help));
        } else if let Some(draft) = &form.hermes {
            fields = fields.push(super::hermes::fields(form, draft, help));
        } else if form.hermes_config_unavailable {
            fields = fields.push(
                text("此档案的 Hermes 节点不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if let Some(draft) = &form.pi {
            fields = fields.push(super::pi::fields(form, draft, help));
        } else if form.pi_config_unavailable {
            fields = fields.push(
                text("此档案的 Pi 节点不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if let Some(draft) = &form.kimi {
            fields = fields.push(super::kimi::fields(form, draft, help));
        } else if form.kimi_config_unavailable {
            fields = fields.push(
                text("此档案的 Kimi 节点不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if let Some(draft) = &form.mcode {
            fields = fields.push(super::mcode::fields(form, draft, help));
        } else if form.mcode_config_unavailable {
            fields = fields.push(
                text("此档案的 Mcode 节点不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if let Some(draft) = &form.agy {
            fields = fields.push(super::agy::fields(form, draft, help));
        } else if let Some(draft) = &form.dsh {
            fields = fields.push(super::dsh::fields(form, draft, help));
        } else if form.dsh_config_unavailable {
            fields = fields.push(
                text("此档案的 Dsh 节点不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if let Some(draft) = &form.omp {
            fields = fields.push(super::omp::fields(form, draft, help));
        } else if form.omp_config_unavailable {
            fields = fields.push(
                text("此档案的 OMP 节点不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if let Some(draft) = &form.openclaw {
            fields = fields.push(super::openclaw::fields(form, draft, help));
        } else if form.openclaw_config_unavailable {
            fields = fields.push(
                text("此档案的 OpenClaw 节点不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if let Some(draft) = &form.opencode {
            fields = fields.push(super::opencode::fields(form, draft, help));
        } else if form.opencode_config_unavailable {
            fields = fields.push(
                text("此档案的 OpenCode 节点不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if form.grok_config_unavailable {
            fields = fields.push(
                text("此档案的 Grok TOML 不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if form.codex_config_unavailable {
            fields = fields.push(
                text("此档案的 Codex TOML 不可编辑，可修改基础资料。")
                    .style(settings_muted_text_style),
            );
        } else {
            let mut credentials = column![].spacing(8);
            if !matches!(form.mode, Mode::Create) {
                credentials = credentials
                    .push(
                        text("默认保留执行端密钥，界面不读取原值。")
                            .size(13)
                            .style(settings_muted_text_style),
                    )
                    .push(
                        checkbox(form.replace_key)
                            .label("替换密钥")
                            .style(settings_checkbox_style)
                            .on_toggle_maybe(
                                (!form.saving).then_some(|v| message(EditMessage::Replace(v))),
                            ),
                    )
                    .push(
                        checkbox(form.clear_key)
                            .label("明确清除密钥（必需凭据将由执行端拒绝）")
                            .style(settings_checkbox_style)
                            .on_toggle_maybe(
                                (!form.saving).then_some(|v| message(EditMessage::Clear(v))),
                            ),
                    );
            }
            if form.replace_key || matches!(form.mode, Mode::Create) {
                credentials = credentials
                    .push(settings_field_label("新的完整密钥"))
                    .push(input("输入新密钥", &form.key, EditMessage::Key).secure(true));
            }
            if matches!(form.mode, Mode::Create) && !form.draft_ready() {
                credentials = credentials.push(
                    text("待补完整应用配置：请先完成应用专属表单，保存暂不可用。")
                        .size(13)
                        .style(settings_muted_text_style),
                );
            }
            fields = fields.push(text("凭据").size(17)).push(settings_panel(credentials));
        }
    }
    let enabled = !form.saving
        && (delete
            || if form.claude_editable()
                || form.gemini_editable()
                || form.codex.is_some()
                || form.grok.is_some()
                || form.opencode.is_some()
                || form.hermes.is_some()
                || form.pi.is_some()
                || form.kimi.is_some()
                || form.mcode.is_some()
                || form.agy.is_some()
                || form.dsh.is_some()
                || form.omp.is_some()
                || form.openclaw.is_some()
            {
                form.identified()
            } else {
                form.request().is_ok()
            });
    let cancel = message(EditMessage::Cancel);
    let header = settings_modal_header(text(title).size(20), settings_close_button(cancel.clone()));
    let actions = row![
        button(text("取消"))
            .padding([8, 16])
            .style(rounded_action_btn_style)
            .on_press_maybe((!form.saving).then_some(cancel.clone())),
        button(text(if form.saving {
            "处理中…"
        } else if delete {
            "确认删除"
        } else {
            "保存"
        }))
        .padding([8, 16])
        .style(primary_action_btn_style)
        .on_press_maybe(enabled.then_some(message(EditMessage::Save))),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let card = settings_modal_layout(header, fields, Some(actions.into()))
        .width(620)
        .height(if delete { 300 } else { 620 });
    settings_modal_overlay(Some(base), cancel, container(card).padding(12))
}
