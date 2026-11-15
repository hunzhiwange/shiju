use desktop_ui_controls::system_settings_common::*;
#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Close,
    Load,
    Edit(Option<String>),
    Cancel,
    Slug(String),
    Body(iced::widget::text_editor::Action),
    Save,
    Delete(String),
    Undo,
    Copy(String),
}
// Pi native template drafts; the server owns parsing, naming and revision checks.
use gateway_app_prompt_resources::{PiPromptResourcesScope, PiPromptTemplate};
use iced::{
    Element,
    widget::{button, column, row, text, text_editor},
};

#[derive(Debug, Clone)]
pub struct Draft {
    pub original: Option<PiPromptTemplate>,
    pub slug: String,
    pub edited: bool,
    pub body: text_editor::Content,
}
#[derive(Debug, Clone, Default)]
pub struct State {
    pub open: bool,
    pub scope: Option<PiPromptResourcesScope>,
    pub generation: u64,
    pub busy: bool,
    pub loaded: bool,
    pub entries: Vec<PiPromptTemplate>,
    pub draft: Option<Draft>,
    pub undo: Option<PiPromptTemplate>,
    pub delete: Option<String>,
    pub error: Option<String>,
}
pub fn overlay<'a, M: Clone + 'static>(
    state: &'a State,
    base: Element<'a, M>,
    message: fn(super::Event) -> M,
) -> Element<'a, M> {
    let message_event = move |event| message(super::Event::Template(event));
    if !state.open {
        return base;
    }
    let close = message_event(Event::Close);
    let mut body = column![text("Pi 原生模板 · 保存后请在 Pi 执行 /reload")].spacing(10);
    if let Some(error) = &state.error {
        body = body.push(text(error));
    }
    if state.busy {
        body = body.push(text("正在请求目标主机…"));
    }
    let mut footer = row![].spacing(10);
    if let Some(draft) = &state.draft {
        let mut slug = settings_text_input("命令名称", &draft.slug);
        let mut editor = settings_text_editor(&draft.body).font(iced::Font::DEFAULT).height(300);
        if !state.busy {
            slug = slug.on_input(move |s| message_event(Event::Slug(s)));
            editor = editor.on_action(move |a| message_event(Event::Body(a)));
        }
        body = body
            .push(text("命令名称（可改名）"))
            .push(slug)
            .push(text("原文（保留 frontmatter 与变量）"))
            .push(editor);
        footer = footer
            .push(
                button(text("返回列表"))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!state.busy).then(|| message_event(Event::Cancel))),
            )
            .push(
                button(text("保存"))
                    .padding([8, 12])
                    .style(primary_action_btn_style)
                    .on_press_maybe((!state.busy).then(|| message_event(Event::Save))),
            );
    } else {
        for entry in &state.entries {
            body = body
                .push(text(format!("/{}", entry.slug)))
                .push(text(entry.summary.argument_hint.as_deref().unwrap_or("")))
                .push(text(entry.summary.description.as_deref().unwrap_or("")))
                .push(
                    row![
                        button(text("编辑 / 改名"))
                            .padding([8, 12])
                            .style(rounded_action_btn_style)
                            .on_press_maybe(
                                (!state.busy)
                                    .then(|| message_event(Event::Edit(Some(entry.slug.clone()))))
                            ),
                        button(text("复制命令"))
                            .padding([8, 12])
                            .style(rounded_action_btn_style)
                            .on_press(message_event(Event::Copy(entry.slug.clone()))),
                        button(text(if state.delete.as_ref() == Some(&entry.slug) {
                            "确认删除"
                        } else {
                            "删除"
                        }))
                        .padding([8, 12])
                        .style(rounded_action_btn_style)
                        .on_press_maybe(
                            (!state.busy).then(|| message_event(Event::Delete(entry.slug.clone())))
                        ),
                    ]
                    .spacing(8),
                );
        }
        footer = footer
            .push(
                button(text("刷新"))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!state.busy).then(|| message_event(Event::Load))),
            )
            .push(
                button(text("新增模板"))
                    .padding([8, 12])
                    .style(primary_action_btn_style)
                    .on_press_maybe(
                        (!state.busy && state.loaded).then(|| message_event(Event::Edit(None))),
                    ),
            );
        if state.undo.is_some() {
            footer = footer.push(
                button(text("撤销删除"))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!state.busy).then(|| message_event(Event::Undo))),
            );
        }
    }
    let header =
        settings_modal_header(text("Pi 原生模板").size(20), settings_close_button(close.clone()));
    settings_modal_overlay(
        Some(base),
        close,
        settings_modal_layout(header, body, Some(footer.into())).width(860).height(680),
    )
}
