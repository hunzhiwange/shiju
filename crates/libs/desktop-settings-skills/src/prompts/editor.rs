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
    page(draft, "提示词", None, message)
}

pub fn page<'a, M: Clone + 'static>(
    draft: &'a Draft,
    application: &str,
    target: Option<&str>,
    message: fn(super::Event) -> M,
) -> Element<'a, M> {
    use desktop_assets::assets::Icon;
    use iced::widget::{Space, container};
    let close = message(Event::CloseEditor);
    let mut name = settings_text_input("例如 团队编码规范", &draft.entry.name);
    let mut description =
        settings_text_input("可选", draft.entry.description.as_deref().unwrap_or(""));
    let placeholder = target
        .and_then(|path| path.rsplit(['/', '\\']).find(|part| !part.is_empty()))
        .map(|filename| format!("# {filename}\n\n用 Markdown 编写给 {application} 的长期说明…"))
        .unwrap_or_else(|| "用 Markdown 编写提示词…".into());
    let mut body = settings_text_editor(&draft.body).placeholder(placeholder).height(Length::Fill);
    if !draft.saving {
        name = name.on_input(move |v| message(Event::Name(v)));
        description = description.on_input(move |v| message(Event::Description(v)));
        body = body.on_action(move |v| message(Event::Body(v)));
    }
    let content = draft.body.text();
    let bytes = content.len();
    let characters = content.chars().count();
    let count = if draft.mcode {
        format!("{characters} 个字符 · {bytes} / 32768 B")
    } else {
        format!("{characters} 个字符 · {bytes} B")
    };
    let header = row![
        icon_btn(Icon::ChevronLeft, "返回提示词", (!draft.saving).then_some(close.clone())),
        text(format!(
            "{} · {application}",
            if draft.updating { "编辑提示词" } else { "添加提示词" }
        ))
        .size(19)
        .font(iced::Font { weight: iced::font::Weight::Semibold, ..iced::Font::DEFAULT }),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center);
    let mut fields = column![
        column![
            row![
                text("名称").size(14),
                text("*").size(14).style(|theme: &iced::Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().danger.base.color),
                }),
            ]
            .spacing(4),
            name,
        ]
        .spacing(6),
        column![text("说明").size(14), description].spacing(6),
        column![
            text("正文").size(14),
            body,
            row![
                Space::new().width(Length::Fill),
                text(count).size(12).style(settings_muted_text_style)
            ],
        ]
        .spacing(6)
        .height(Length::Fill),
    ]
    .spacing(14)
    .height(Length::Fill);
    if application == "Pi" {
        fields = fields.push(text("更改后请在 Pi 执行 /reload。"));
    } else if application == "Hermes" {
        fields = fields.push(text("Hermes 提示词在新会话中生效。"));
    }
    let mut footer = column![].spacing(8);
    if let Some(error) = &draft.error {
        footer = footer.push(settings_error_banner(error.clone()));
    }
    let submit = if draft.saving {
        "保存中…"
    } else if draft.updating {
        "保存"
    } else {
        "添加"
    };
    footer = footer.push(
        row![
            Space::new().width(Length::Fill),
            button(container(text("取消").size(14)).center_y(Length::Fill))
                .height(34)
                .padding([0, 15])
                .style(rounded_action_btn_style)
                .on_press_maybe((!draft.saving).then_some(close)),
            button(container(text(submit).size(14)).center_y(Length::Fill))
                .height(34)
                .padding([0, 15])
                .style(primary_action_btn_style)
                .on_press_maybe((!draft.saving).then_some(message(Event::Save))),
        ]
        .spacing(10),
    );
    column![
        container(header).padding([10, 18]).width(Length::Fill),
        settings_divider(),
        container(fields).padding([16, 18]).width(Length::Fill).height(Length::Fill),
        settings_divider(),
        container(footer).padding([10, 18]).width(Length::Fill),
    ]
    .height(Length::Fill)
    .into()
}
