//! Drafts remain owned by the dialog until a matching save succeeds.
use crate::app::{Message, components::system_settings_common::*, message::settings::prompts::{Event, message}};
use iced::{Element, Length, widget::{button, column, row, text, text_editor}};
use vw_gateway_client::AgentAppPromptEntry;

#[derive(Debug, Clone)]
pub(crate) struct Draft {
    pub entry: AgentAppPromptEntry,
    pub body: text_editor::Content,
    pub updating: bool,
    pub revision: u64,
    pub saving: bool,
    pub error: Option<String>,
    pub mcode: bool,
}
impl Draft {
    pub fn new(entry: Option<AgentAppPromptEntry>, revision: u64, mcode: bool) -> Self {
        let updating = entry.is_some();
        let entry = entry.unwrap_or(AgentAppPromptEntry { id: String::new(), name: String::new(), content: String::new(), description: None, created_at: None });
        let body = text_editor::Content::with_text(&entry.content);
        Self { entry, body, updating, revision, saving: false, error: None, mcode }
    }
}
pub(crate) fn view<'a>(draft: &'a Draft, base: Element<'a, Message>) -> Element<'a, Message> {
    let close = message(Event::CloseEditor);
    let mut name = settings_text_input("名称", &draft.entry.name);
    let mut description = settings_text_input("描述（可选）", draft.entry.description.as_deref().unwrap_or(""));
    let mut body = settings_text_editor(&draft.body).font(iced::Font::DEFAULT).height(320);
    if !draft.saving {
        name = name.on_input(|v| message(Event::Name(v)));
        description = description.on_input(|v| message(Event::Description(v)));
        body = body.on_action(|v| message(Event::Body(v)));
    }
    let bytes = draft.body.text().len();
    let mut fields = column![text("名称"), name, text("描述"), description, text("正文"), body,
        text(if draft.mcode { format!("{bytes} / 32768 UTF-8 字节（服务端限长）") } else { format!("{bytes} UTF-8 字节") })].spacing(10);
    if let Some(error) = &draft.error { fields = fields.push(text(error)); }
    let footer = row![
        button(text("取消")).padding([8, 16]).style(rounded_action_btn_style).on_press_maybe((!draft.saving).then_some(close.clone())),
        button(text(if draft.saving { "保存中…" } else { "保存" })).padding([8, 16]).style(primary_action_btn_style).on_press_maybe((!draft.saving).then_some(message(Event::Save)))
    ].spacing(10);
    let header = settings_modal_header(text(if draft.updating { "编辑提示词" } else { "新增提示词" }).size(20), settings_close_button(close.clone()));
    settings_modal_overlay(Some(base), close, settings_modal_layout(header, fields, Some(footer.into())).width(Length::Fixed(720.0)).height(Length::Fill))
}
