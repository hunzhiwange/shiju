use desktop_ui_controls::system_settings_common::*;
#[derive(Clone, Debug)]
pub enum FileEvent {
    Open,
    Close,
    Select(usize),
    Edit(iced::widget::text_editor::Action),
    Read,
    Save,
    Delete,
    AcceptRevision,
}
// Revision-bound editors for the two external Pi system resources.
use gateway_app_prompt_resources::{
    PiPromptFileKind, PiPromptFileSnapshot, PiPromptResourcesScope,
};
use iced::{
    Element,
    widget::{button, column, row, text, text_editor},
};

#[derive(Debug, Default)]
pub struct FileDraft {
    pub base: Option<PiPromptFileSnapshot>,
    pub draft: text_editor::Content,
    pub latest: Option<PiPromptFileSnapshot>,
    pub blocked: bool,
    pub busy: bool,
    pub error: Option<String>,
    pub confirm_delete: bool,
    pub confirm_override: bool,
}
// Preserve an independent draft when the surrounding settings state is cloned.
impl Clone for FileDraft {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            draft: text_editor::Content::with_text(&self.draft.text()),
            latest: self.latest.clone(),
            blocked: self.blocked,
            busy: self.busy,
            error: self.error.clone(),
            confirm_delete: self.confirm_delete,
            confirm_override: self.confirm_override,
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct State {
    pub open: bool,
    pub scope: Option<PiPromptResourcesScope>,
    pub generation: u64,
    pub selected: usize,
    pub files: [FileDraft; 2],
}
pub fn kind(index: usize) -> PiPromptFileKind {
    match index {
        0 => PiPromptFileKind::SystemAppend,
        _ => PiPromptFileKind::SystemOverride,
    }
}
pub fn overlay<'a, M: Clone + 'static>(
    state: &'a State,
    base: Element<'a, M>,
    message: fn(super::Event) -> M,
) -> Element<'a, M> {
    let message_event = move |event| message(super::Event::PiFile(event));
    if !state.open {
        return base;
    }
    let file = &state.files[state.selected];
    let busy = state.files.iter().any(|file| file.busy);
    let close = message_event(FileEvent::Close);
    let header =
        settings_modal_header(text("Pi 系统文件").size(20), settings_close_button(close.clone()));
    let mut body = column![
        text("外部 Pi 系统文件；不会拼接识矩运行时系统提示词。"),
        row![
            settings_tab_button(
                "追加 · APPEND_SYSTEM.md",
                state.selected == 0,
                (!busy).then(|| message_event(FileEvent::Select(0)))
            ),
            settings_tab_button(
                "覆盖 · SYSTEM.md",
                state.selected == 1,
                (!busy).then(|| message_event(FileEvent::Select(1)))
            )
        ]
        .spacing(8),
        text(match &file.base {
            None => "尚未读取",
            Some(snapshot) if snapshot.exists => "文件存在",
            Some(_) => "文件不存在",
        }),
    ]
    .spacing(12);
    if let Some(error) = &file.error {
        body = body.push(text(error));
    }
    if file.busy {
        body = body.push(text("正在请求目标主机…"));
    }
    let mut editor = settings_text_editor(&file.draft).height(260);
    if !busy && file.base.is_some() {
        editor = editor.on_action(move |action| message_event(FileEvent::Edit(action)));
    }
    body = body.push(editor);
    if file.blocked {
        body = body.push(text("写入未确认。草稿已保留，请重新读取并比较当前文件后显式确认。"));
    }
    if let Some(latest) = &file.latest {
        body = body
            .push(text(if latest.exists {
                "主机当前内容（草稿保留在上方）"
            } else {
                "主机当前文件不存在"
            }))
            .push(text(latest.content.clone()))
            .push(
                button(text("已比较，使用当前 revision 保留草稿继续编辑"))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!busy).then(|| message_event(FileEvent::AcceptRevision))),
            );
    }
    if file.confirm_delete {
        body = body.push(text("确认删除当前文件？未保存的草稿将保留。"));
    }
    if file.confirm_override {
        body = body.push(text("创建 SYSTEM.md 会覆盖 Pi 默认系统提示词，请再次点击保存确认。"));
    }
    let writable = !busy && file.base.is_some() && !file.blocked;
    let footer = row![
        button(text("重新读取 / 比较"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!busy).then(|| message_event(FileEvent::Read))),
        button(text(if file.confirm_delete { "确认删除" } else { "删除" }))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe(
                (writable && file.base.as_ref().is_some_and(|s| s.exists))
                    .then(|| message_event(FileEvent::Delete))
            ),
        button(text("取消"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!busy).then_some(close.clone())),
        button(text("保存")).padding([8, 12]).style(primary_action_btn_style).on_press_maybe(
            (writable && !file.draft.text().trim().is_empty())
                .then(|| message_event(FileEvent::Save))
        ),
    ]
    .spacing(10);
    let card = settings_modal_layout(header, body, Some(footer.into())).width(860).height(680);
    settings_modal_overlay(Some(base), close, card)
}
