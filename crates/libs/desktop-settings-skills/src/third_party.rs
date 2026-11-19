//! Third-party skill entry. The host routes scoped directory actions to existing
//! modals and executes binding effects with the same scope attached to replies.
/// Page identity uses the gateway and real project ID; directory is UI context only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    pub context: desktop_provider_apps::AgentAppsContext,
    pub project: Option<String>,
    pub host: String,
    pub directory: Option<String>,
}
pub mod workspace;
use crate::update::{Input, bindings_view as bindings};
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Element, Length,
    widget::{button, checkbox, column, row, text},
};

#[derive(Clone, Copy, Debug)]
pub enum DirectoryAction {
    Repositories,
    Claude,
    Import,
    Install,
    Market,
    Updates,
    Jobs,
    Storage,
    Backup,
}

#[derive(Clone, Debug)]
pub enum Action {
    Directory(DirectoryAction),
    Binding(bindings::Event),
    Workspace(workspace::Event),
}

/// Carry this envelope through asynchronous completion; never rebuild its scope
/// from the currently selected application when a response arrives.
#[derive(Clone, Debug)]
pub struct Message {
    pub scope: Scope,
    pub catalog: u64,
    pub action: Action,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    scope: Option<Scope>,
    catalog: u64,
    pub bindings: bindings::State,
    pub workspace: workspace::State,
}

pub enum Effect {
    None,
    Workspace { scope: Scope, catalog: u64, effect: workspace::Effect },
    Directory { scope: Scope, catalog: u64, action: DirectoryAction },
    Binding { scope: Scope, catalog: u64, effect: bindings::Effect },
}

impl State {
    /// Synchronize before rendering and dispatch. Native skill state is never
    /// borrowed or mutated by this entry.
    pub fn sync(&mut self, scope: Option<Scope>, catalog: u64) {
        if self.scope != scope || self.catalog != catalog {
            self.bindings = bindings::State::default();
            self.workspace = workspace::State::default();
            self.scope = scope;
            self.catalog = catalog;
        }
    }

    pub fn scope(&self) -> Option<&Scope> {
        self.scope.as_ref()
    }
}

pub fn update(state: &mut State, input: &Input, message: Message) -> Effect {
    if state.scope.as_ref() != Some(&message.scope)
        || input.context.as_ref() != Some(&message.scope.context)
        || input.host != message.scope.host
        || state.catalog != message.catalog
        || state.catalog != input.catalog
        || input.project != message.scope.project
        || input.directory != message.scope.directory
    {
        return Effect::None;
    }
    let scope = message.scope;
    let event = match message.action {
        Action::Workspace(event) => {
            if input.job_active
                && matches!(
                    event,
                    workspace::Event::Install(_) | workspace::Event::ConfirmRepository
                )
            {
                state.workspace.notice = Some("请先在任务进度中确认当前技能任务的最终状态".into());
                return Effect::None;
            }
            return Effect::Workspace {
                scope,
                catalog: state.catalog,
                effect: workspace::update(&mut state.workspace, event, input.repository_version),
            };
        }
        Action::Binding(event) => event,
        Action::Directory(action) => {
            state.workspace.add_menu = false;
            state.workspace.more_menu = false;
            return Effect::Directory { scope, catalog: state.catalog, action };
        }
    };
    // Global preferences contain other applications. This page deliberately
    // changes bindings only, leaving shared preference editing to its owner.
    if matches!(event, bindings::Event::Save) {
        return Effect::Binding {
            scope,
            catalog: state.catalog,
            effect: bindings::Effect::Error("请在全局应用绑定中保存目标偏好".into()),
        };
    }
    if let bindings::Event::Target(target, _) = &event {
        if !input.targets.contains(target) {
            return Effect::None;
        }
    }
    if matches!(event, bindings::Event::Bind(_) | bindings::Event::Resync) {
        let supported = state.bindings.snapshot.as_ref().is_some_and(|snapshot| {
            state.bindings.targets.iter().all(|target| {
                snapshot
                    .capabilities
                    .iter()
                    .any(|capability| capability.app_id == target.app_id && capability.supported)
            })
        });
        if !supported
            || state.bindings.targets.is_empty()
            || state.bindings.targets.iter().any(|target| !input.targets.contains(target))
        {
            return Effect::Binding {
                scope,
                catalog: state.catalog,
                effect: bindings::Effect::Error(
                    "当前应用或安装实例不支持技能绑定，请刷新应用目录".into(),
                ),
            };
        }
    }
    if let bindings::Event::Select(id) = &event {
        if !state.bindings.open {
            let effect = bindings::update(&mut state.bindings, input, bindings::Event::Open);
            state.bindings.selected = Some(id.clone());
            return Effect::Binding { scope, catalog: state.catalog, effect };
        }
    }
    let effect = bindings::update(&mut state.bindings, input, event);
    Effect::Binding { scope, catalog: state.catalog, effect }
}

/// Explicit application target plus shared host directory controls. Installation
/// remains a host-directory operation; it does not implicitly enable a binding.
pub fn view<'a>(
    state: &'a State,
    applications: &[crate::bindings_view::BindingApplication<'_>],
) -> Element<'a, Message> {
    let Some(scope) = state.scope.as_ref() else {
        return settings_error_banner("请打开项目并连接执行主机".to_owned()).into();
    };
    let message = |action| Message { scope: scope.clone(), catalog: state.catalog, action };
    let mut body = column![text(format!("安装到 · 已选 {} 个应用实例", state.bindings.targets.len()))].spacing(6).width(Length::Fill);
    let s = &state.bindings;
    body = body.push(
        button("刷新应用绑定").padding([8, 10]).style(rounded_action_btn_style).on_press_maybe(
            (!s.busy).then(|| {
                message(Action::Binding(if s.open {
                    bindings::Event::Refresh
                } else {
                    bindings::Event::Open
                }))
            }),
        ),
    );
    for application in applications {
        let app_id = application.entry.id;
        let capability = s
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.capabilities.iter().find(|item| item.app_id == app_id));
        if !capability.is_some_and(|item| item.supported) {
            continue;
        }
        body = body.push(text(application.entry.name.clone()));
        if let Some(observation) = application.observation {
            for installation in
                observation.installations.iter().filter(|item| item.target_host_id == scope.host)
            {
                let target = vw_api_app_skills::bindings_api::SkillBindingTarget {
                    app_id,
                    installation_id: installation.id.0.clone(),
                };
                let selected = s.targets.contains(&target);
                let scope = scope.clone();
                body = body.push(
                    checkbox(selected)
                        .label(installation.id.0.clone())
                        .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                        .style(settings_checkbox_style)
                        .on_toggle_maybe((!s.busy).then_some(move |enabled| Message {
                            scope: scope.clone(),
                            catalog: state.catalog,
                            action: Action::Binding(bindings::Event::Target(
                                target.clone(),
                                enabled,
                            )),
                        })),
                );
            }
        }
    }
    let supported = s.snapshot.as_ref().is_some_and(|snapshot| {
        s.targets.iter().all(|target| {
            snapshot.capabilities.iter().any(|item| item.app_id == target.app_id && item.supported)
        })
    });
    let can_bind = supported && !s.busy && !s.targets.is_empty();
    let mut controls = row![].spacing(8);
    for (label, event, enabled) in [
        ("启用所选", bindings::Event::Bind(true), can_bind && s.selected.is_some()),
        ("停用所选", bindings::Event::Bind(false), can_bind && s.selected.is_some()),
        ("重新同步", bindings::Event::Resync, can_bind),
        ("取消选择", bindings::Event::Close, !s.busy),
    ] {
        controls = controls.push(
            button(label)
                .padding([8, 10])
                .style(rounded_action_btn_style)
                .on_press_maybe(enabled.then(|| message(Action::Binding(event)))),
        );
    }
    body = body.push(controls);
    if s.busy {
        body = body.push(text("正在读取或写入，请稍候…"));
    }
    for result in &s.results {
        body = body.push(text(format!(
            "{} / {}：{:?}",
            result.target.installation_id, result.directory, result.status
        )));
    }
    if let Some(error) = &s.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    workspace::view(state, iced::widget::scrollable(body).height(150)
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4))).into())
}
