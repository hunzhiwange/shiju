use super::Event;
use desktop_ui_controls::system_settings_common::*;
use gateway_app_prompts::{AgentAppPromptCatalog, AgentAppPromptsScope};
use iced::{
    Element,
    widget::{button, column, text},
};
use vw_api_app_operations::PromptTargetPreview;
use vw_api_types::agent_apps::AppId;

#[derive(Clone, Debug)]
pub struct State<C> {
    pub query: String,
    pub menu: Option<Menu>,
    pub sync: super::sync::State<C>,
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
impl<C> State<C> {
    pub fn invalidate(&mut self) {
        let mut sync = std::mem::take(&mut self.sync);
        sync.open = false;
        *self = Self { sync, generation: self.generation.wrapping_add(1), ..Self::default() };
    }
}
/// Only one action surface can be open in the current scope.
#[derive(Clone, Debug, PartialEq)]
pub enum Menu { Page, Entry(String), Help }

pub fn view<'a, M: Clone + 'static, C: Clone + 'a>(
    state: &'a State<C>, id: AppId, message: fn(super::Event) -> M,
) -> Element<'a, M> {
    use iced::{Length, widget::{container, row, text_input, mouse_area}};
    use super::transfer::TransferEvent as Transfer;
    if let Some(draft) = &state.editor {
        return super::editor::page(draft, &format!("{id:?}"), message);
    }
    let ready = !state.loading && !state.mutating && state.editor.is_none();
    let loaded = state.scope.as_ref().is_some_and(|s| s.app_id == id) && state.catalog.is_some();
    let action = |label: &str, event: Event, enabled: bool| {
        button(text(label.to_owned())).padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe(enabled.then(|| message(event)))
    };
    let mut content = column![row![
        text("智能提示词").size(22),
        action("帮助", Event::Menu(Some(Menu::Help)), true),
        button(text("添加提示词")).padding([8, 12]).style(primary_action_btn_style)
            .on_press_maybe((ready && loaded).then(|| message(Event::Edit(None)))),
        action("更多", Event::Menu(Some(Menu::Page)), ready),
    ].spacing(8).wrap()].spacing(12);
    let status = if state.loading { "正在读取…" }
        else if state.error.is_some() { "读取失败，请刷新重试" }
        else if let Some(preview) = &state.preview {
            match preview.snapshot.content.as_deref() {
                None => "文件缺失", Some("") => "空文件", Some(_) => "已有内容",
            }
        } else { "尚未读取" };
    content = content.push(row![text(format!("当前应用：{id:?}")), text(status)].spacing(12).wrap());
    if let Some(preview) = &state.preview {
        content = content.push(text(preview.display_path.as_deref().unwrap_or("目标路径不可用")));
    }
    if let Some(error) = &state.error { content = content.push(text(error)); }
    if let Some(catalog) = state.catalog.as_ref().filter(|_| loaded) {
        if !state.loading && !catalog.entries.is_empty() {
            content = content.push(row![
                text_input("搜索名称、说明或正文", &state.query)
                    .on_input(move |v| message(Event::Query(v)))
                    .style(settings_text_input_style).padding(SETTINGS_CONTROL_PADDING)
                    .size(SETTINGS_CONTROL_TEXT_SIZE),
                action("清除", Event::Query(String::new()), !state.query.is_empty()),
            ].spacing(8));
        }
        let query = state.query.trim().to_lowercase();
        let mut visible = 0;
        for entry in &catalog.entries {
            if ![Some(entry.name.as_str()), entry.description.as_deref(), Some(entry.content.as_str())]
                .into_iter().flatten().any(|v| v.to_lowercase().contains(&query)) { continue; }
            visible += 1;
            let active = catalog.active_id.as_ref() == Some(&entry.id);
            let summary: String = entry.description.as_deref().filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| entry.content.lines().find(|v| !v.trim().is_empty()).unwrap_or("无正文"))
                .chars().take(100).collect();
            let mut details = column![text(&entry.name).size(17), text(summary),
                text(format!("{} UTF-8 字节", entry.content.len())).size(12)].spacing(5);
            if let Some(created) = entry.created_at { details = details.push(text(format!("创建时间：{created}")).size(12)); }
            let mutation = if active { super::activation::Action::Deactivate(entry.id.clone()) }
                else { super::activation::Action::Activate(entry.id.clone()) };
            content = content.push(container(row![details.width(Length::Fill),
                column![text(if state.mutating { "处理中…" } else if active { "已启用" } else { "已停用" }),
                    row![action(if active { "停用" } else { "启用" }, Event::Mutate(mutation), ready && state.preview.is_some()),
                        action("编辑", Event::Edit(Some(entry.id.clone())), ready),
                        action("更多", Event::Menu(Some(Menu::Entry(entry.id.clone()))), ready)].spacing(6).wrap()
                ].spacing(6)
            ].spacing(12)).padding(12).width(Length::Fill));
        }
        if visible == 0 {
            let empty = if catalog.entries.is_empty() { format!("{id:?} 暂无提示词") } else { "没有匹配的提示词".to_owned() };
            content = content.push(container(column![text(empty),
                if catalog.entries.is_empty() { action("添加提示词", Event::Edit(None), ready) }
                else { action("清除搜索", Event::Query(String::new()), true) }
            ].spacing(12).align_x(iced::Alignment::Center)).center_x(Length::Fill).padding(32));
        }
    }
    if state.undo.is_some() { content = content.push(action("撤销上次操作", Event::Undo, ready)); }
    if id == AppId::Pi { content = content.push(text("更改后请在 Pi 执行 /reload。")); }
    if id == AppId::Hermes { content = content.push(text("Hermes 提示词在新会话中生效。")); }
    let mut base: Element<'a, M> = content.into();
    if let Some(menu) = &state.menu {
        let mut items = column![].spacing(8);
        let title = match menu {
            Menu::Help => {
                items = items.push(text("提示词按当前主机和应用保存。启用会写入目标文件；复制到其他应用后保持停用。"));
                "提示词帮助"
            }
            Menu::Entry(entry_id) => {
                items = items.push(action("复制到其他应用…", Event::Transfer(Transfer::Open(entry_id.clone())), ready))
                    .push(action("复制正文", Event::Transfer(Transfer::Body(entry_id.clone())), ready))
                    .push(action("删除", Event::Mutate(super::activation::Action::Delete(entry_id.clone())), ready && state.preview.is_some()));
                "条目操作"
            }
            Menu::Page => {
                let importable = ready && loaded && state.preview.as_ref().is_some_and(|p| p.snapshot.content.as_ref().is_some_and(|c| !c.trim().is_empty()));
                items = items.push(action("刷新", Event::Open(id), ready))
                    .push(action("导入当前文件", Event::Transfer(Transfer::ImportHost), importable));
                if !importable { items = items.push(text("当前文件未读取、缺失或为空，暂不可导入。")); }
                items = items.push(action("从本机选择文件…", Event::Transfer(Transfer::ImportClient), ready && loaded));
                let path = ready && state.preview.as_ref().is_some_and(|p| p.display_path.is_some());
                items = items.push(action("打开本机所在位置 / 复制远端路径", Event::Transfer(Transfer::Reveal), path))
                    .push(action("复制路径", Event::Transfer(Transfer::Path), path))
                    .push(action("全部同步 / 查看进度", Event::Sync(super::sync::Event::Open), ready))
                    .push(action("识矩系统提示词", Event::Agents, ready))
                    .push(action("项目规则", Event::Projects, ready));
                if id == AppId::Pi {
                    items = items.push(action("Pi 原生模板", Event::Template(super::templates::Event::Open), ready))
                        .push(action("Pi 系统文件", Event::PiFile(super::pi_files::FileEvent::Open), ready));
                }
                "更多操作"
            }
        };
        let close = message(Event::Menu(None));
        let card = settings_modal_layout(settings_modal_header(text(title).size(20), settings_close_button(close.clone())), items, None).width(520);
        base = settings_modal_overlay_with_escape_and_backdrop_dismiss(base, close.clone(), card);
        // The front layer consumes the wheel before the modal scrollable or page sees it.
        let wheel = mouse_area(container(text("")).width(Length::Fill).height(Length::Fill))
            .on_scroll(move |_| close.clone());
        base = iced::widget::stack![base, wheel].into();
    }
    let base = super::sync::overlay(&state.sync, base, message);
    let base = super::shared_import::overlay(state, base, message);
    base
}

impl<C> Default for State<C> {
    fn default() -> Self {
        Self {
            query: String::new(),
            menu: None,
            sync: Default::default(),
            templates: Default::default(),
            shared_import: Default::default(),
            transfer: Default::default(),
            pi_files: Default::default(),
            scope: Default::default(),
            generation: Default::default(),
            loading: Default::default(),
            catalog: Default::default(),
            preview: Default::default(),
            error: Default::default(),
            mutating: Default::default(),
            undo: Default::default(),
            editor: Default::default(),
        }
    }
}
