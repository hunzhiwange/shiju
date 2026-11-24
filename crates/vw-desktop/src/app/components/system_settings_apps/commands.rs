//! Copy-only commands and authorized installation diagnostics.
use app_lifecycle::manual_commands::{commands_for, ManualAction, ManualMethod, ManualPlatform};
use crate::app::{App, Message, state::AppsCatalogState, message::settings::AppsCatalogMessage};
use desktop_ui_controls::system_settings_common::*;
use iced::{Element, Length, Task, widget::{button, column, container, row, text}};
use vw_api_app_identity::{AppInstallationEnvironment as Environment, AppInstallationSource as Source};

const PLATFORMS: [&str; 4] = ["Windows · PowerShell", "macOS · Bash", "Linux · Bash", "WSL · 发行版内 Bash"];
const METHODS: [&str; 7] = ["npm", "pnpm", "Bun", "Homebrew Formula", "Homebrew Cask", "原生安装器", "Hermes"];
const PLATFORM_VALUES: [ManualPlatform; 4] = [ManualPlatform::Windows, ManualPlatform::MacOs, ManualPlatform::Linux, ManualPlatform::Wsl];
const METHOD_VALUES: [ManualMethod; 7] = [ManualMethod::Npm, ManualMethod::Pnpm, ManualMethod::Bun, ManualMethod::BrewFormula, ManualMethod::BrewCask, ManualMethod::Native, ManualMethod::Hermes];

#[derive(Clone, Debug)]
pub enum Event {
    Open(u8), Close, Platform(usize), Method(usize), Copy(usize), Reset(usize, u64),
    Diagnosed { request: u64, host: String, context: crate::app::state::AgentAppsContext,
        result: Result<Vec<super::catalog::CatalogRow>, String> },
}
fn event(value: Event) -> Message { super::message(AppsCatalogMessage::Commands(value)) }

fn recipes(state: &AppsCatalogState) -> Vec<(String, String)> {
    let options = &state.commands;
    let action = if options.panel == Some(1) { ManualAction::Install } else { ManualAction::Uninstall };
    state.rows.iter().flat_map(|row| {
        commands_for(row.entry.id, PLATFORM_VALUES[options.platform], METHOD_VALUES[options.method], action)
            .into_iter().map(|command| {
                let label = match command.variant {
                    Some(variant) => format!("{} · {variant}", row.entry.name),
                    None => row.entry.name.clone(),
                };
                (label, command.text)
            })
    }).collect()
}

pub(crate) fn update(app: &mut App, value: Event) -> Task<Message> {
    match value {
        Event::Open(panel) => {
            if !(1..=3).contains(&panel) { return Task::none(); }
            let selected = app.apps_catalog.rows.iter()
                .find(|row| Some(row.entry.id) == app.apps_catalog.expanded)
                .and_then(|row| row.observation.as_ref().ok())
                .and_then(|observation| observation.installations.iter().find(|i| i.is_path_default)
                    .or_else(|| observation.installations.first()));
            let defaults = selected.map(|installation| {
                let platform = match installation.environment {
                    Environment::Windows => 0, Environment::Macos => 1,
                    Environment::Linux => 2, Environment::Wsl { .. } => 3,
                };
                let method = match installation.source {
                    Source::Pnpm => Some(1), Source::Bun => Some(2),
                    Source::Native => Some(5), _ => None,
                };
                (platform, method)
            });
            let state = &mut app.apps_catalog.commands;
            state.invalidate();
            state.panel = Some(panel);
            // No target-host evidence: use an explicitly labelled, switchable default.
            state.platform = defaults.map(|v| v.0).unwrap_or(2);
            state.method = defaults.and_then(|v| v.1).unwrap_or(0);
            state.inferred = defaults.is_some_and(|v| v.1.is_some());
            if panel == 3 {
                state.diagnostics.clear();
                state.diagnostics_error = None;
                let client = match crate::app::config::gateway_client() {
                    Ok(client) => client,
                    Err(_) => {
                        state.diagnostics_error = Some("网关不可用，请重新连接".into());
                        return app.show_localized_toast(crate::app::state::ToastKind::Error, "网关不可用，请重新连接", vec![]);
                    }
                };
                let context = crate::app::state::AgentAppsContext {
                    gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(),
                    project: app.navigation_tabs.project_path.clone(),
                };
                let host = app.apps_catalog.host.clone();
                let token = app.apps_catalog.token.clone();
                let request = state.sequence;
                state.diagnostics_loading = true;
                return Task::perform(
                    desktop_settings_apps::system_settings_apps::catalog_requests::load(client, host.clone(), token),
                    move |result| event(Event::Diagnosed { request, host: host.clone(), context: context.clone(), result }),
                );
            }
        }
        Event::Diagnosed { request, host, context, result } => {
            let current = crate::app::config::gateway_client().ok().map(|client| crate::app::state::AgentAppsContext {
                gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(),
                project: app.navigation_tabs.project_path.clone(),
            });
            let state = &mut app.apps_catalog.commands;
            if state.panel != Some(3) || request != state.sequence || host != app.apps_catalog.host
                || current.as_ref() != Some(&context) { return Task::none(); }
            state.diagnostics_loading = false;
            match result {
                Ok(rows) => state.diagnostics = rows.into_iter().map(|row| {
                    use desktop_i18n::format::Message as Copy;
                    let mut lines = Vec::new();
                    match row.observation {
                        Err(error) => lines.push(Copy::nested("探测失败：%s".into(), vec![
                            Copy::new(super::catalog::error_label(&error), vec![]),
                        ])),
                        Ok(observation) => {
                            lines.push(Copy::new(if observation.installations.len() > 1 {
                                "发现多个安装实例，请核对环境与路径优先级"
                            } else if observation.installations.is_empty() { "未知：未发现 CLI 安装实例" }
                            else { "无冲突：仅发现一个安装实例" }, vec![]));
                            for installation in observation.installations {
                                lines.push(Copy::nested("%s · %s · %s".into(), vec![
                                    Copy::literal(format!("{:?}", installation.environment)),
                                    Copy::literal(format!("{:?}", installation.source)),
                                    Copy::new(if installation.is_path_default { "PATH 默认" }
                                        else { "非 PATH 默认（具体顺序未知）" }, vec![]),
                                ]));
                                lines.push(Copy::literal(installation.path));
                                if let Some(path) = installation.real_path {
                                    lines.push(Copy::new("真实路径：%s", vec![path.into()]));
                                }
                                lines.push(Copy::new("探测状态：%s", vec![format!("{:?}", installation.runtime_status).into()]));
                            }
                        }
                    }
                    (row.entry.name, lines)
                }).collect(),
                Err(error) => {
                    state.diagnostics_error = Some(error.clone());
                    return app.show_error_toast(error);
                }
            }
        }
        Event::Close => app.apps_catalog.commands.invalidate(),
        Event::Platform(index) if index < PLATFORMS.len() => {
            let state = &mut app.apps_catalog.commands;
            state.clear_feedback(); state.platform = index; state.inferred = false;
        }
        Event::Method(index) if index < METHODS.len() => {
            let state = &mut app.apps_catalog.commands;
            state.clear_feedback(); state.method = index; state.inferred = false;
        }
        Event::Copy(index) => {
            if !matches!(app.apps_catalog.commands.panel, Some(1 | 2)) { return Task::none(); }
            let rows = recipes(&app.apps_catalog);
            let content = if index == usize::MAX {
                rows.iter().map(|(_, command)| command.as_str()).collect::<Vec<_>>().join("\n\n")
            } else if let Some((_, command)) = rows.get(index) { command.clone() }
            else { return Task::none(); };
            if content.is_empty() { return Task::none(); }
            let state = &mut app.apps_catalog.commands;
            state.sequence = state.sequence.wrapping_add(1);
            let request = state.sequence;
            state.copied.insert(index, request);
            return Task::batch([
                iced::clipboard::write(content),
                crate::app::message::after(std::time::Duration::from_secs(2), event(Event::Reset(index, request))),
            ]);
        }
        Event::Reset(index, request) => {
            let state = &mut app.apps_catalog.commands;
            if state.copied.get(&index) == Some(&request) { state.copied.remove(&index); }
        }
        _ => {}
    }
    Task::none()
}

pub(crate) fn overlay<'a>(i18n: desktop_i18n::I18n, state: &'a AppsCatalogState, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(panel) = state.commands.panel else { return base; };
    let title = match panel { 1 => i18n.__("手动安装命令"), 2 => i18n.__("手动卸载命令"), _ => i18n.__("诊断安装冲突") };
    let mut body = column![].spacing(14);
    if panel == 3 {
        if state.commands.diagnostics_loading { body = body.push(text(i18n.__("正在重新检查安装状态…"))); }
        else if let Some(error) = &state.commands.diagnostics_error { body = body.push(settings_error_banner(error.clone())); }
        else {
            if state.commands.diagnostics.is_empty() { body = body.push(text(i18n.__("未知：尚无安装观察"))); }
            for (name, lines) in &state.commands.diagnostics {
                let mut details = column![text(name.clone()).size(17)].spacing(8);
                for line in lines { details = details.push(text(line.render(i18n).expect("valid installation diagnostic template")).size(13)); }
                body = body.push(settings_panel(details));
            }
        }
    } else {
        body = body.push(text(if state.commands.inferred { i18n.__("默认选项来自所选应用的安装观察，可切换。") }
            else { i18n.__("选项未完全由安装观察确认。缺少环境证据时默认 Linux，缺少方式证据时默认 npm；请切换至实际环境和方式。") }).size(13))
            .push(row![
                settings_select(PLATFORMS, Some(PLATFORMS[state.commands.platform]), |value| event(Event::Platform(PLATFORMS.iter().position(|v| *v == value).unwrap_or(2)))).format_labels(|value| i18n.__(value).to_owned()).width(Length::Fill),
                settings_select(METHODS, Some(METHODS[state.commands.method]), |value| event(Event::Method(METHODS.iter().position(|v| *v == value).unwrap_or(0)))).format_labels(|value| i18n.__(value).to_owned()).width(Length::Fill),
            ].spacing(12));
        let recipes = recipes(state);
        if recipes.is_empty() { body = body.push(text(i18n.__("此组合暂无已审核命令，请切换运行环境或方式。"))); }
        for (index, (label, command)) in recipes.into_iter().enumerate() {
            body = body.push(settings_panel(column![
                row![text(label).width(Length::Fill), copy_button(i18n, state, index)],
                text(command).size(13),
            ].spacing(8)));
        }
    }
    let mut actions = row![].spacing(8);
    if panel == 1 { actions = actions.push(copy_button(i18n, state, usize::MAX)); }
    if panel == 3 { actions = actions.push(button(text(i18n.__("重新诊断"))).padding([8, 12]).style(rounded_action_btn_style)
        .on_press_maybe((!state.commands.diagnostics_loading).then_some(event(Event::Open(3))))); }
    actions = actions.push(button(text(i18n.__("关闭"))).padding([8, 12]).style(rounded_action_btn_style).on_press(event(Event::Close)));
    let card = settings_modal_layout_with_scroll_id(
        settings_modal_header(text(title).size(24), settings_close_button(event(Event::Close))),
        body, Some(actions.into()), Some("apps-manual-commands-body"),
    ).width(Length::Fill).max_width(760.).max_height(620.);
    settings_modal_overlay(Some(base), event(Event::Close), container(card).padding(12).center(Length::Fill))
}
fn copy_button(i18n: desktop_i18n::I18n, state: &AppsCatalogState, index: usize) -> Element<'static, Message> {
    button(text(if state.commands.copied.contains_key(&index) { i18n.__("已复制") }
        else if index == usize::MAX { i18n.__("整体复制") } else { i18n.__("复制") }))
        .padding([8, 12]).style(rounded_action_btn_style).on_press(event(Event::Copy(index))).into()
}
