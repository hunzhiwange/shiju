//! Third-party MCP entry point. The desktop host owns navigation and scrolling.
use crate::management_bindings::BindingsMessage;
use crate::management_context::Context;
use crate::management_update::Update;
use desktop_ui_controls::system_settings_common::*;
use iced::widget::{checkbox, column, row, text};
use iced::{Element, Length, Task};
use vw_api_app_identity::{AppId, AppInstallation, AppInstallationEnvironment};
use vw_api_mcp_management::*;

/// Include the host navigation generation so queued actions cannot cross a reopened page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub context: Context,
    pub app_id: AppId,
    pub generation: u64,
}

impl Scope {
    pub fn contains(&self, target: &McpAppTarget) -> bool {
        target.app_id == self.app_id && target.target_host_id == self.context.host
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub scope: Scope,
    pub action: Action,
}

#[derive(Debug, Clone)]
pub enum Action {
    Refresh,
    Bindings(BindingsMessage),
}

/// Reuses the existing catalog/binding state and effects; never touches runtime selection.
/// The host must route returned messages here even after navigating away, to settle writes.
pub fn update(app: &mut Update<'_>, current: &Scope, message: Message) -> Task<Message> {
    let origin = message.scope;
    if app.context.as_ref() != Some(&origin.context) {
        return Task::none();
    }
    // A completed write still settles its original request after app navigation. The
    // existing controller checks context and request generation; the view filters targets.
    let completion = matches!(&message.action, Action::Bindings(BindingsMessage::Finished { .. }));
    if !completion && &origin != current {
        return Task::none();
    }
    let binding = match message.action {
        Action::Refresh => return app.refresh(),
        Action::Bindings(binding) => binding,
    };
    match &binding {
        BindingsMessage::Apply(target, _) if !origin.contains(target) => {
            return app.error("发布目标与当前第三方应用或主机不一致");
        }
        BindingsMessage::RetryFailed => {
            let valid = matches!(app.state.bindings.result.as_ref(), Some(Ok(result))
                if !result.items.is_empty() && result.items.iter().all(|item| origin.contains(&item.target)));
            if !valid {
                return app.error("请在原目标应用中重试失败项");
            }
        }
        BindingsMessage::Finished { result: Ok(result), .. }
            if result.items.iter().any(|item| !origin.contains(&item.target)) =>
        {
            return app.error("发布结果目标与请求应用或主机不一致");
        }
        _ => {}
    }
    crate::management_bindings_update::update(app, binding)
        .map(move |binding| Message { scope: origin.clone(), action: Action::Bindings(binding) })
}

pub struct View<'a> {
    pub scope: &'a Scope,
    pub state: &'a crate::management_state::McpSettingsState,
    /// Trusted installation observations from the existing Apps catalog.
    pub installations: &'a [AppInstallation],
    pub authorized: bool,
}

pub fn view(input: View<'_>) -> Element<'_, Message> {
    let scope = input.scope;
    let message = |action| Message { scope: scope.clone(), action };
    let catalog = &input.state.catalog;
    let visible = catalog.context.as_ref() == Some(&scope.context);
    let loading = visible && catalog.loading;
    let bindings = crate::management_bindings::visible(&input.state.bindings, Some(&scope.context));
    let selected = bindings.map(|state| state.selected.as_slice()).unwrap_or(&[]);
    let running = input.state.bindings.running;
    let mut body = column![
        text("第三方 MCP").size(20),
        text(format!("目标应用：{:?} · 主机：{}", scope.app_id, scope.context.host)).size(14),
        text("目录为共享连接定义；启用或禁用仅发布到下方指定安装实例。识矩运行时与 ACP 的选择独立管理。")
            .size(12).style(settings_muted_text_style),
        crate::toolbar_button("刷新目录", (!loading).then(|| message(Action::Refresh))),
    ].spacing(10).width(Length::Fill);
    let result = visible.then_some(catalog.result.as_ref()).flatten();
    let catalog = match result {
        Some(Ok(catalog)) => catalog,
        Some(Err(error)) => {
            return settings_panel(body.push(settings_error_banner(error.clone()))).into();
        }
        None => {
            return settings_panel(
                body.push(
                    text(if loading {
                        "正在加载共享 MCP 目录"
                    } else {
                        "请先连接目标主机并刷新共享 MCP 目录"
                    })
                    .style(settings_muted_text_style),
                ),
            )
            .into();
        }
    };
    body = body.push(text("共享 MCP 目录").size(16));
    if catalog.records.is_empty() {
        body = body.push(text("当前目录为空").style(settings_muted_text_style));
    }
    for record in &catalog.records {
        let record = record.clone();
        let origin = scope.clone();
        body = body.push(
            checkbox(selected.contains(&record))
                .label(format!(
                    "{} · {:?} · 修订 {}",
                    record.name, record.transport, record.record.revision
                ))
                .style(settings_checkbox_style)
                .on_toggle_maybe((!running && !loading).then_some(move |value| Message {
                    scope: origin.clone(),
                    action: Action::Bindings(BindingsMessage::Select(record.clone(), value)),
                })),
        );
    }
    if let Some(reason) = crate::distribution::unsupported_reason(scope.app_id) {
        return settings_panel(body.push(text(reason).style(settings_muted_text_style))).into();
    }
    let supported = crate::distribution::supported(catalog, selected, scope.app_id);
    let enabled = input.authorized
        && !running
        && !loading
        && supported
        && selected.iter().all(|record| catalog.records.contains(record))
        && crate::distribution::project(selected).is_ok();
    if !input.authorized {
        body = body.push(text("请先授权连接目标主机").style(settings_muted_text_style));
    }
    if !selected.is_empty() && !supported {
        body = body.push(
            text("所选连接的传输不受目标应用支持或能力尚未验证").style(settings_muted_text_style),
        );
    }
    let mut found = false;
    for installation in input.installations.iter().filter(|installation| {
        installation.app_id == scope.app_id
            && installation.target_host_id == scope.context.host
            && matches!(installation.environment, AppInstallationEnvironment::Macos)
    }) {
        found = true;
        let target = McpAppTarget {
            app_id: scope.app_id,
            installation_id: installation.id.clone(),
            target_host_id: scope.context.host.clone(),
        };
        body = body.push(
            row![
                text(format!("安装实例：{}", installation.id.0)).size(13),
                crate::toolbar_button(
                    "启用并发布",
                    enabled.then(|| message(Action::Bindings(BindingsMessage::Apply(
                        target.clone(),
                        true
                    ))))
                ),
                crate::toolbar_button(
                    "禁用并发布",
                    enabled.then(|| message(Action::Bindings(BindingsMessage::Apply(
                        target.clone(),
                        false
                    ))))
                ),
            ]
            .spacing(8)
            .wrap(),
        );
        for binding in catalog.bindings.iter().filter(|binding| binding.target == target) {
            body = body.push(
                text(format!(
                    "{} · 修订 {} · 期望{} · 原生 {:?}（上次观察）",
                    binding.record.id.0,
                    binding.record.revision,
                    if binding.enabled { "启用" } else { "禁用" },
                    binding.native_status
                ))
                .size(12)
                .style(settings_muted_text_style),
            );
        }
    }
    if !found {
        body = body.push(
            text("尚无当前主机的可信 macOS 安装实例；请在应用管理中刷新，其它平台尚不支持发布。")
                .size(12)
                .style(settings_muted_text_style),
        );
    }
    if running {
        body = body.push(text("正在等待发布结果...").style(settings_muted_text_style));
    }
    if let Some(Ok(result)) = bindings.and_then(|state| state.result.as_ref()) {
        for item in result.items.iter().filter(|item| scope.contains(&item.target)) {
            body = body.push(text(crate::distribution::item_label(item)).size(12));
        }
    } else if let Some(Err(error)) = bindings.and_then(|state| state.result.as_ref()) {
        body = body.push(settings_error_banner(error.clone()));
    }
    settings_panel(body).into()
}
