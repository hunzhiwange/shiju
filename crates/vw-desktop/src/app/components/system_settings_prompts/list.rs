use crate::app::{Message, components::system_settings_common::*, message::settings::prompts::{Event, message}};
use iced::{Element, widget::{button, column, text}};
use vw_gateway_client::{AgentAppPromptCatalog, AgentAppPromptsScope, vw_api_types::agent_apps::{AppId, PromptTargetPreview}};

#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    pub sync: super::sync::State,
    pub templates: super::templates::State,
    pub shared_import: Option<super::shared_import::Draft>,
    pub transfer: super::transfer::State,
    pub pi_files: super::pi_files::State,
    pub scope: Option<AgentAppPromptsScope>,
    pub generation: u64,
    pub loading: bool,
    pub catalog: Option<AgentAppPromptCatalog>,
    pub preview: Option<PromptTargetPreview>,
    pub error: Option<String>,
    pub mutating: bool,
    pub undo: Option<super::activation::Undo>,
    pub editor: Option<super::editor::Draft>,
}
impl State {
    pub fn invalidate(&mut self) {
        let mut sync = std::mem::take(&mut self.sync);
        sync.open = false;
        *self = Self { sync, generation: self.generation.wrapping_add(1), ..Self::default() };
    }
}
pub(crate) fn view(state: &State, id: AppId) -> Element<'_, Message> {
    use crate::app::{components::system_settings::SystemTab, message::SettingsMessage};
    let navigate = |tab| Message::Settings(SettingsMessage::SystemTabSelected(tab));
    let mut content = column![
        button(text("全部同步 / 查看进度")).padding([8, 12]).style(rounded_action_btn_style).on_press(message(Event::Sync(crate::app::message::settings::prompts::sync::Event::Open))),
        text("识矩系统提示词").size(15),
        button(text("打开智能体提示词编辑入口")).padding([8, 12]).style(rounded_action_btn_style).on_press(navigate(SystemTab::Agents)),
        text("项目规则").size(15),
        button(text("打开项目设置")).padding([8, 12]).style(rounded_action_btn_style).on_press(navigate(SystemTab::Projects)),
        text("外部应用提示词与模板").size(15),
    ].spacing(10);
    if id == AppId::Pi { content = content.push(button(text("管理 Pi 原生模板")).padding([8, 12]).style(rounded_action_btn_style).on_press(message(Event::Template(crate::app::message::settings::prompts::templates::Event::Open)))); }
    if id == AppId::Pi { content = content.push(button(text("编辑 Pi 系统文件")).padding([8, 12]).style(rounded_action_btn_style).on_press(message(Event::PiFile(crate::app::message::settings::prompts::pi_files::Event::Open)))); }
    if id == AppId::OpenClaw {
        return super::sync::overlay(&state.sync, content.push(text("OpenClaw 请沿用工作区文件入口管理，不在此处创建副本。")).into());
    }
    if id == AppId::ClaudeDesktop {
        content = content.push(text("Claude Desktop 不提供此入口，请显式选择 Claude Code。"))
            .push(button(text("查看 Claude Code 提示词")).padding([8, 12]).style(rounded_action_btn_style).on_press(message(Event::Open(AppId::Claude))));
        if state.scope.as_ref().is_none_or(|scope| scope.app_id != AppId::Claude) { return super::sync::overlay(&state.sync, content.into()); }
    }
    content = content.push(button(text("加载 / 刷新当前应用提示词")).padding([8, 12]).style(rounded_action_btn_style).on_press(message(Event::Open(id))));
    if state.scope.as_ref().is_none_or(|scope| scope.app_id != id && !(id == AppId::ClaudeDesktop && scope.app_id == AppId::Claude)) { return super::sync::overlay(&state.sync, content.into()); }
    if state.loading { content = content.push(text("正在加载当前目标…")); }
    if let Some(error) = &state.error { content = content.push(text(error)); }
    let transfer_event = |event| message(Event::Transfer(event));
    use crate::app::message::settings::prompts::transfer::Event as TransferEvent;
    let ready = !state.loading && !state.mutating && state.editor.is_none();
    if state.catalog.is_some() {
        content = content.push(button(text("导入当前目标主机文件")).padding([8, 12]).style(rounded_action_btn_style).on_press_maybe(ready.then(|| transfer_event(TransferEvent::ImportHost))))
            .push(button(text("从用户本机选择文件导入")).padding([8, 12]).style(rounded_action_btn_style).on_press_maybe(ready.then(|| transfer_event(TransferEvent::ImportClient))));
    }
    if let Some(catalog) = &state.catalog {
        content = content.push(button(text("新增提示词")).padding([8, 12]).style(primary_action_btn_style).on_press(message(Event::Edit(None))));
        if catalog.entries.is_empty() { content = content.push(text("暂无提示词条目")); }
        for entry in &catalog.entries {
            let active = catalog.active_id.as_ref() == Some(&entry.id);
            content = content.push(button(text(format!("{}{} · 编辑", entry.name, if active { " · 当前激活" } else { "" }))).padding([8, 12]).style(rounded_action_btn_style).on_press(message(Event::Edit(Some(entry.id.clone())))));
            let action = if active { super::activation::Action::Deactivate(entry.id.clone()) }
                else { super::activation::Action::Activate(entry.id.clone()) };
            let enabled = !state.loading && !state.mutating && state.editor.is_none() && state.preview.is_some();
            content = content.push(button(text(if active { "停用" } else { "激活" }))
                .padding([8, 12]).style(rounded_action_btn_style)
                .on_press_maybe(enabled.then(|| message(Event::Mutate(action)))))
                .push(button(text("删除")).padding([8, 12]).style(rounded_action_btn_style)
                .on_press_maybe(enabled.then(|| message(Event::Mutate(super::activation::Action::Delete(entry.id.clone()))))));
            content = content.push(button(text("复制正文")).padding([8, 12]).style(rounded_action_btn_style).on_press_maybe(ready.then(|| transfer_event(TransferEvent::Body(entry.id.clone())))))
                .push(button(text("复制到其他应用…")).padding([8, 12]).style(rounded_action_btn_style).on_press_maybe(ready.then(|| transfer_event(TransferEvent::Open(entry.id.clone())))));
            if let Some(description) = &entry.description { content = content.push(text(description)); }
            content = content.push(text(&entry.content));
        }
    }
    if state.undo.is_some() {
        content = content.push(button(text("撤销上次操作")).padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.loading && !state.mutating).then(|| message(Event::Undo))));
    }
    if let Some(scope) = &state.scope {
        match scope.app_id {
            AppId::Pi => content = content.push(text("更改后请在 Pi 执行 /reload。")),
            AppId::Hermes => content = content.push(text("Hermes 提示词在新会话中生效。")),
            _ => {}
        }
    }
    if let Some(preview) = &state.preview {
        content = content.push(button(text("复制完整路径")).padding([8, 12]).style(rounded_action_btn_style).on_press_maybe((ready && preview.display_path.is_some()).then(|| transfer_event(TransferEvent::Path))))
            .push(button(text("定位本机文件 / 复制远端路径")).padding([8, 12]).style(rounded_action_btn_style).on_press_maybe((ready && preview.display_path.is_some()).then(|| transfer_event(TransferEvent::Reveal))))
            .push(text("目标文件预览").size(15))
            .push(text(preview.display_path.as_deref().unwrap_or("目标路径不可用")))
            .push(text(match preview.snapshot.content.as_deref() { None => "文件不存在", Some("") => "文件为空", Some(content) => content }));
    }
    let base = super::sync::overlay(&state.sync, content.into());
    let base = super::shared_import::overlay(state, base);
    match &state.editor {
        Some(draft) => super::editor::view(draft, base),
        None => base,
    }
}
