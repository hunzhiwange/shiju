use super::Event;
use desktop_ui_controls::system_settings_common::*;
// Drafts remain owned by the page until a matching save succeeds.
use gateway_app_prompts::AgentAppPromptEntry;
use iced::{
    Element, Length,
    widget::{button, column, row, text, text_editor},
};

#[derive(Debug, Clone)]
pub struct Draft {
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
        let entry = entry.unwrap_or(AgentAppPromptEntry {
            id: String::new(),
            name: String::new(),
            content: String::new(),
            description: None,
            created_at: None,
        });
        let body = text_editor::Content::with_text(&entry.content);
        Self { entry, body, updating, revision, saving: false, error: None, mcode }
    }
}
// Retain the existing adapter for callers that supply a base surface.
pub fn view<'a, M: Clone + 'static>(
    draft: &'a Draft,
    _base: Element<'a, M>,
    message: fn(super::Event) -> M,
) -> Element<'a, M> {
    page(draft, "提示词", message)
}

pub fn page<'a, M: Clone + 'static>(
    draft: &'a Draft,
    application: &str,
    message: fn(super::Event) -> M,
) -> Element<'a, M> {
    use iced::widget::{Space, container};
    let close = message(Event::CloseEditor);
    let mut name = settings_text_input("名称（必填）", &draft.entry.name);
    let mut description =
        settings_text_input("说明（可选）", draft.entry.description.as_deref().unwrap_or(""));
    let mut body = settings_text_editor(&draft.body).height(Length::Fill);
    if !draft.saving {
        name = name.on_input(move |v| message(Event::Name(v)));
        description = description.on_input(move |v| message(Event::Description(v)));
        body = body.on_action(move |v| message(Event::Body(v)));
    }
    let content = draft.body.text();
    let bytes = content.len();
    let characters = content.chars().count();
    let count = if draft.mcode {
        format!("{characters} 字符 · {bytes} / 32768 UTF-8 字节")
    } else {
        format!("{characters} 字符 · {bytes} UTF-8 字节")
    };
    let header = row![
        button(text("返回"))
            .padding([8, 16]).style(rounded_action_btn_style)
            .on_press_maybe((!draft.saving).then_some(close.clone())),
        text(format!("{} · {application}", if draft.updating { "编辑提示词" } else { "新增提示词" })).size(20),
    ].spacing(12).align_y(iced::Alignment::Center);
    let mut fields = column![text("名称 *"), name, text("说明"), description,
        text("正文 · Markdown"), body,
        row![Space::new().width(Length::Fill), text(count).size(12)],
    ].spacing(10).height(Length::Fill);
    if application == "Pi" {
        fields = fields.push(text("更改后请在 Pi 执行 /reload。"));
    } else if application == "Hermes" {
        fields = fields.push(text("Hermes 提示词在新会话中生效。"));
    }
    let mut footer = column![].spacing(8);
    if let Some(error) = &draft.error {
        footer = footer.push(settings_error_banner(error.clone()));
    }
    footer = footer.push(row![Space::new().width(Length::Fill),
        button(text("取消")).padding([8, 16]).style(rounded_action_btn_style)
            .on_press_maybe((!draft.saving).then_some(close)),
        button(text(if draft.saving { "保存中…" } else { "保存" }))
            .padding([8, 16]).style(primary_action_btn_style)
            .on_press_maybe((!draft.saving).then_some(message(Event::Save))),
    ].spacing(10));
    container(column![header, fields, footer].spacing(12).height(Length::Fill))
        .width(Length::Fill).height(Length::Fill).into()
}
