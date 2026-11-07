use super::super::system_settings_common::*;
use crate::app::{
    Message,
    message::app_providers::edit::{EditMessage, message},
    state::app_providers::edit::{EditForm, Mode},
};
use iced::{
    Element, Length,
    widget::{button, checkbox, column, container, row, scrollable, text, text_input},
};
pub(crate) fn overlay<'a>(form: &'a EditForm, base: Element<'a, Message>) -> Element<'a, Message> {
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
    let mut fields = column![].spacing(10);
    let delete = matches!(form.mode, Mode::Delete);
    if delete {
        fields = fields.push(text(format!(
            "确认删除「{}」？执行端会检查当前选择与引用，失败时保留此选择。",
            form.fields.name
        )));
    } else {
        if matches!(form.mode, Mode::Create | Mode::Copy { .. }) {
            fields = fields.push(text("新标识（ASCII 字母、数字、短横线或下划线）")).push(input(
                "新标识",
                &form.id,
                EditMessage::Id,
            ));
        }
        fields =
            fields.push(text("名称")).push(input("名称", &form.fields.name, EditMessage::Name));
        if !form.claude_editable() {
            fields = fields.push(text("分类")).push(input(
                "分类",
                form.fields.category.as_deref().unwrap_or(""),
                EditMessage::Category,
            ));
        }
        fields = fields
            .push(text("备注"))
            .push(input("备注", form.fields.notes.as_deref().unwrap_or(""), EditMessage::Notes))
            .push(text("网址（展示网址，与 API 地址分开保存）"))
            .push(input(
                "https://…",
                form.fields.website_url.as_deref().unwrap_or(""),
                EditMessage::Website,
            ));
        if let Some(draft) = &form.claude {
            fields = fields.push(super::claude::fields(form, draft));
        } else if let Some(draft) = &form.codex {
            fields = fields.push(super::codex::fields(form, draft));
        } else if let Some(draft) = &form.gemini {
            fields = fields.push(super::gemini::fields(form, draft));
        } else if let Some(draft) = &form.grok {
            fields = fields.push(super::grok::fields(form, draft));
        } else if let Some(draft) = &form.hermes {
            fields = fields.push(super::hermes::fields(form, draft));
        } else if form.hermes_config_unavailable {
            fields = fields.push(
                text("此档案的 Hermes 节点不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if let Some(draft) = &form.pi {
            fields = fields.push(super::pi::fields(form, draft));
        } else if form.pi_config_unavailable {
            fields = fields.push(
                text("此档案的 Pi 节点不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if let Some(draft) = &form.kimi {
            fields = fields.push(super::kimi::fields(form, draft));
        } else if form.kimi_config_unavailable {
            fields = fields.push(
                text("此档案的 Kimi 节点不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if let Some(draft) = &form.mcode {
            fields = fields.push(super::mcode::fields(form, draft));
        } else if form.mcode_config_unavailable {
            fields = fields.push(
                text("此档案的 Mcode 节点不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if let Some(draft) = &form.agy {
            fields = fields.push(super::agy::fields(form, draft));
        } else if let Some(draft) = &form.dsh {
            fields = fields.push(super::dsh::fields(form, draft));
        } else if form.dsh_config_unavailable {
            fields = fields.push(
                text("此档案的 Dsh 节点不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if let Some(draft) = &form.omp {
            fields = fields.push(super::omp::fields(form, draft));
        } else if form.omp_config_unavailable {
            fields = fields.push(
                text("此档案的 OMP 节点不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if let Some(draft) = &form.openclaw {
            fields = fields.push(super::openclaw::fields(form, draft));
        } else if form.openclaw_config_unavailable {
            fields = fields.push(
                text("此档案的 OpenClaw 节点不可编辑，可修改基础资料。官方授权暂不可用。")
                    .style(settings_muted_text_style),
            );
        } else if let Some(draft) = &form.opencode {
            fields = fields.push(super::opencode::fields(form, draft));
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
            if !matches!(form.mode, Mode::Create) {
                fields = fields
                    .push(text("默认保留执行端密钥，界面不读取原值。"))
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
                fields = fields
                    .push(text("新的完整密钥"))
                    .push(input("输入新密钥", &form.key, EditMessage::Key).secure(true));
            }
            if matches!(form.mode, Mode::Create) && !form.draft_ready() {
                fields =
                    fields.push(text("待补完整应用配置：请先完成应用专属表单，保存暂不可用。"));
            }
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
    let card = settings_modal_card(
        column![
            row![
                text(title).size(22).width(Length::Fill),
                settings_close_button(message(EditMessage::Cancel))
            ],
            scrollable(container(fields).padding([0, 7]))
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(4).scroller_width(4)
                ))
                .height(Length::Fill),
            row![
                button(text("取消"))
                    .padding([8, 16])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!form.saving).then_some(message(EditMessage::Cancel))),
                button(text(if form.saving {
                    "处理中…"
                } else if delete {
                    "确认删除"
                } else {
                    "保存"
                }))
                .padding([8, 16])
                .style(primary_action_btn_style)
                .on_press_maybe(enabled.then_some(message(EditMessage::Save)))
            ]
            .spacing(12)
        ]
        .spacing(16),
    )
    .width(Length::Fixed(620.))
    .height(Length::Fixed(620.));
    settings_modal_overlay(Some(base), message(EditMessage::Cancel), card)
}
