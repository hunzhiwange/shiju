use super::super::system_settings_common::*;
use crate::app::{
    Message,
    message::app_providers::snippets::{Event, message},
    state::app_providers::{
        list::ListState,
        snippets::{Form, Operation},
    },
};
use iced::{
    Element, Length,
    widget::{button, column, row, text, text_editor},
};
use vw_gateway_client::vw_api_types::agent_apps::AppId;

#[derive(Clone, PartialEq, Eq)]
struct ProviderOption {
    id: String,
    name: String,
}

impl std::fmt::Display for ProviderOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

fn action(
    label: &'static str,
    event: Event,
    enabled: bool,
    primary: bool,
) -> Element<'static, Message> {
    button(text(label))
        .padding([8, 14])
        .style(if primary { primary_action_btn_style } else { rounded_action_btn_style })
        .on_press_maybe(enabled.then(|| message(event)))
        .into()
}

pub(super) fn overlay<'a>(
    state: &'a ListState,
    form: &'a Form,
    app: AppId,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let ready = form.pending.is_none();
    let can_close = form.pending != Some(Operation::Save);
    let close = if can_close { message(Event::Close) } else { Message::None };
    let header = settings_modal_header(
        settings_page_intro("公共配置片段", "保存后更新应用的公共片段。"),
        settings_close_button(close.clone()),
    );
    let mut body = column![].spacing(12);
    if form.loaded {
        let options: Vec<_> = state
            .snapshot
            .iter()
            .flat_map(|snapshot| &snapshot.catalog.rows)
            .map(|row| ProviderOption {
                id: row.provider.id.clone(),
                name: row.provider.name.clone(),
            })
            .collect();
        if !options.is_empty() {
            let selected = options
                .iter()
                .find(|option| Some(&option.id) == form.provider_id.as_ref())
                .cloned();
            body = body.push(text("从已保存的供应商提取").size(16)).push(
                row![
                    settings_select(options, selected, |option| message(Event::SelectProvider(
                        option.id
                    )))
                    .placeholder("选择供应商")
                    .width(Length::Fill)
                    .disabled(!ready),
                    action("提取", Event::Extract, ready && form.provider_id.is_some(), false),
                ]
                .spacing(8),
            );
        }
        let mut editor = text_editor(&form.draft)
            .height(Length::Fixed(240.))
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_editor_style_for(ready));
        if ready {
            editor = editor.on_action(|action| message(Event::Edit(action)));
        }
        body = body
            .push(
                text(if app == AppId::Codex {
                    "片段草稿 · TOML"
                } else {
                    "片段草稿 · JSON"
                })
                .size(16),
            )
            .push(
                text("留空可清除已保存的片段。预览展示服务端校验和规范化后的内容。")
                    .size(13)
                    .style(settings_muted_text_style),
            )
            .push(editor);
    } else if ready {
        body = body.push(action("重试读取", Event::RetryRead, true, false));
    }
    if let Some(operation) = form.pending {
        let progress = match operation {
            Operation::Read => "正在读取公共片段…",
            Operation::Extract => "正在提取公共片段…",
            Operation::Preview => "正在校验草稿…",
            Operation::Save => "正在保存公共片段…",
        };
        body = body.push(text(progress).size(13));
    }
    if let Some(error) = &form.error {
        body = body.push(settings_panel(text(format!("请求失败　{error}")).size(13)));
    }
    if let Some((operation, snippet)) = &form.preview {
        let extracted = *operation == Operation::Extract;
        let mut result = column![
            text(if extracted { "提取预览" } else { "服务预览" }).size(16),
            text(snippet.content.as_deref().unwrap_or("（空片段）")).size(14),
        ]
        .spacing(8)
        .width(Length::Fill);
        if extracted {
            result = result.push(action("用提取结果替换草稿", Event::AdoptExtracted, ready, false));
        }
        body = body.push(settings_panel(result));
    }
    let footer = row![
        action("取消", Event::Close, can_close, false),
        action("服务预览", Event::Preview, ready && form.loaded, false),
        action("保存片段", Event::Save, ready && form.loaded, true),
    ]
    .spacing(10);
    let card = settings_modal_layout(header, body.width(Length::Fill), Some(footer.into()))
        .width(700)
        .height(655);
    settings_modal_overlay(Some(base), close, card)
}
