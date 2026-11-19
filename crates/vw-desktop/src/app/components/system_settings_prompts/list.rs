pub(crate) type State =
    desktop_settings_skills::prompts::list::State<crate::app::state::AgentAppsContext>;
pub(crate) fn view(
    state: &State,
    id: vw_api_app_identity::AppId,
) -> iced::Element<'_, crate::app::Message> {
    let base = desktop_settings_skills::prompts::list::view(state, id, super::ui_message);
    desktop_settings_skills::prompts::transfer::overlay(
        &state.transfer, id, state.mutating || state.loading, base, super::ui_message,
    )
}

use crate::app::{App, Message, message::settings::{AppsCatalogMessage, prompts}};
use desktop_ui_controls::settings_navigation::ThirdPartyPage;
use iced::{Element, Length, Task, widget::{button, column, container, row, scrollable, text}};
use vw_api_app_identity::AppId;
use super::super::system_settings_common::rounded_action_btn_style;

pub(crate) fn switch_blocked(state: &State) -> bool {
    state.mutating || state.editor.is_some() || state.shared_import.is_some()
        || state.transfer.entry.is_some() || state.templates.open || state.pi_files.open
        || state.sync.busy || (state.sync.submitted && !state.sync.terminal())
}

fn available(app: &App, id: AppId) -> bool {
    let state = &app.apps_catalog;
    !state.loading && state.error.is_none() && state.context.is_some() && !state.token.is_empty()
        && (desktop_settings_skills::prompts::transfer::TARGETS.contains(&id)
            || matches!(id, AppId::ClaudeDesktop | AppId::OpenClaw))
        && state.rows.iter().any(|r| r.entry.id == id && r.observation.is_ok())
}

pub(crate) fn select(app: &mut App, id: AppId) -> Task<Message> {
    if switch_blocked(&app.apps_catalog.prompts) || !available(app, id) {
        return Task::none();
    }
    app.apps_catalog.prompts.query.clear();
    app.apps_catalog.prompts.menu = None;
    app.apps_catalog.expanded = Some(id);
    super::super::third_party_settings::navigate(app, ThirdPartyPage::Prompts)
}

pub(crate) fn open_selected(app: &mut App) -> Task<Message> {
    if switch_blocked(&app.apps_catalog.prompts) { return Task::none(); }
    let selected = app.apps_catalog.expanded.filter(|id| available(app, *id))
        .or_else(|| app.apps_catalog.rows.iter().find(|r| available(app, r.entry.id)).map(|r| r.entry.id));
    app.apps_catalog.expanded = selected;
    match selected {
        Some(AppId::ClaudeDesktop | AppId::OpenClaw) | None => Task::none(),
        Some(id) => prompts::update(app, prompts::Event::Open(id)),
    }
}

pub(crate) fn page(app: &App) -> Element<'_, Message> {
    let state = &app.apps_catalog;
    if state.loading { return text("正在读取当前主机的应用目录…").into(); }
    if state.error.is_some() { return text("应用目录读取失败，请刷新目录重试。").into(); }
    let mut menu = column![].spacing(4);
    let mut count = 0;
    let scope_current = state.prompts.scope.as_ref().is_some_and(|scope| {
        scope.target_host_id == state.host
            && scope.project_id.as_ref() == vw_api_transport::local_settings_project(app.workspace_state.project_id.as_deref())
    });
    for item in state.rows.iter().filter(|r| available(app, r.entry.id)) {
        count += 1;
        let id = item.entry.id;
        let selected = state.expanded == Some(id);
        let mut label = row![
            desktop_settings_apps::system_settings_apps::list::brand_icon(id, 18.0),
            text(&item.entry.name).width(Length::Fill),
        ].spacing(8).align_y(iced::Alignment::Center);
        if scope_current && !state.prompts.loading && state.prompts.scope.as_ref().is_some_and(|s| s.app_id == id) {
            if let Some(catalog) = &state.prompts.catalog {
                label = label.push(text(catalog.entries.len().to_string()));
            }
        }
        if selected { label = label.push(text("✓")); }
        menu = menu.push(button(label).padding([8, 12]).width(Length::Fill)
            .style(rounded_action_btn_style)
            .on_press_maybe((!switch_blocked(&state.prompts)).then(|| super::super::system_settings_apps::message(AppsCatalogMessage::SelectPromptApp(id)))));
    }
    if count == 0 { return text("当前主机没有可管理提示词的已授权应用，请连接或刷新应用目录。").into(); }
    let menu = scrollable(container(menu).padding([0, 7]))
        .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Shrink);
    let menu = container(menu).max_height(180).width(Length::Fill);
    let body: Element<'_, Message> = match state.expanded.filter(|id| available(app, *id)) {
        Some(AppId::ClaudeDesktop) => column![text("Claude Desktop 的提示词请在 Claude Code 中管理。"),
            button(text("查看 Claude Code 提示词")).padding([8, 12]).style(rounded_action_btn_style)
                .on_press_maybe(available(app, AppId::Claude).then(|| super::super::system_settings_apps::message(AppsCatalogMessage::SelectPromptApp(AppId::Claude))))].spacing(10).into(),
        Some(AppId::OpenClaw) => column![text("OpenClaw 提示词由工作区文件管理。"), super::super::workspace_files::entry()].spacing(10).into(),
        Some(id) if scope_current => view(&state.prompts, id),
        Some(id) => button(text("加载当前应用提示词")).padding([8, 12]).style(rounded_action_btn_style)
            .on_press(super::super::system_settings_apps::message(AppsCatalogMessage::SelectPromptApp(id))).into(),
        None => text("请选择应用").into(),
    };
    column![menu, body].spacing(16).into()
}
