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
pub(crate) mod style;
pub mod workspace;
use crate::update::{Input, bindings_view as bindings};
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Element, Length,
    widget::{Space, button, checkbox, column, container, row, scrollable, text},
};
use vw_api_app_skills::bindings_api::SkillBindingTarget;
use vw_api_types::agent_apps::AppId;

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
    SelectAllTargets(bool),
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
    custom_targets: bool,
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
            self.custom_targets = false;
            self.scope = scope;
            self.catalog = catalog;
        }
    }

    pub fn available_targets(&self) -> Vec<SkillBindingTarget> {
        self.bindings.snapshot.iter().flat_map(|snapshot| &snapshot.capabilities)
            .filter(|capability| capability.supported)
            .map(|capability| SkillBindingTarget::application(capability.app_id))
            .collect()
    }

    pub fn awaiting_target_snapshot(&self) -> bool {
        !self.custom_targets && self.bindings.snapshot.is_none()
    }

    pub fn reconcile_targets(&mut self, targets: &[SkillBindingTarget]) {
        if self.bindings.snapshot.is_none() {
            return;
        }
        let available = self.available_targets();
        if self.custom_targets {
            self.bindings.targets.retain(|target| available.contains(target));
        } else {
            self.bindings.targets = available.into_iter()
                .filter(|target| targets.iter().any(|installed| installed.app_id == target.app_id))
                .collect();
        }
    }

    pub(super) fn binding_status(
        &self,
        target: &SkillBindingTarget,
        directory: &str,
    ) -> Option<bool> {
        let snapshot = self.bindings.snapshot.as_ref()?;
        let inventory = self.workspace.inventory.as_ref()?;
        if snapshot.scope != inventory.scope || snapshot.version != inventory.version {
            return None;
        }
        if matches!(target.app_id, AppId::Pi | AppId::Omp) {
            return snapshot
                .native_bindings
                .iter()
                .find(|binding| binding.app_id == target.app_id && binding.directory == directory)
                .map(|binding| binding.enabled);
        }
        let bindings = snapshot.managed_bindings.as_ref()?;
        Some(
            bindings
                .iter()
                .any(|binding| binding.target.app_id == target.app_id && binding.directory == directory),
        )
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
        Action::SelectAllTargets(all) => {
            if !state.bindings.busy {
                state.custom_targets = true;
                state.bindings.targets = if all { state.available_targets() } else { Vec::new() };
            }
            return Effect::None;
        }
        Action::Directory(action) => {
            state.workspace.close_menus();
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
    let available = state.available_targets();
    if let bindings::Event::Target(target, enabled) = &event {
        if !state.bindings.busy && available.contains(target) {
            state.custom_targets = true;
            state.bindings.targets.retain(|item| item != target);
            if *enabled {
                state.bindings.targets.push(target.clone());
            }
        }
        return Effect::None;
    }
    if matches!(event, bindings::Event::Bind(_) | bindings::Event::Resync)
        && (state.bindings.targets.is_empty()
            || state.bindings.targets.iter().any(|target| !available.contains(target)))
    {
        return Effect::Binding {
            scope,
            catalog: state.catalog,
            effect: bindings::Effect::Error("当前应用不支持技能绑定，请刷新应用目录".into()),
        };
    }
    if let bindings::Event::BindTarget(id, target, enabled) = &event {
        if !available.contains(target) || state.binding_status(target, id) != Some(!enabled) {
            return Effect::None;
        }
    }
    if let bindings::Event::Select(id) = &event {
        state.workspace.close_menus();
        if !state.bindings.open {
            let effect = bindings::update(&mut state.bindings, input, bindings::Event::Open);
            state.bindings.selected = Some(id.clone());
            return Effect::Binding { scope, catalog: state.catalog, effect };
        }
    }
    if matches!(event, bindings::Event::Close) {
        if !state.bindings.busy {
            state.bindings.selected = None;
            state.bindings.results.clear();
        }
        return Effect::None;
    }
    let preserve_targets = state.custom_targets && matches!(event, bindings::Event::Completed(..));
    let previous = preserve_targets.then(|| state.bindings.targets.clone());
    let scoped_input = Input { targets: available, ..(*input).clone() };
    let effect = bindings::update(&mut state.bindings, &scoped_input, event);
    if let Some(previous) = previous {
        state.bindings.targets = previous;
    }
    state.reconcile_targets(&input.targets);
    Effect::Binding { scope, catalog: state.catalog, effect }
}

pub fn view<'a>(
    state: &'a State,
    applications: &[&vw_api_app_identity::AppCatalogEntry],
    icon: fn(AppId, f32) -> Element<'static, Message>,
) -> Element<'a, Message> {
    let Some(scope) = state.scope.as_ref() else {
        return container(text("请打开项目后重试"))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
    };
    let send = |event| Message {
        scope: scope.clone(),
        catalog: state.catalog,
        action: Action::Binding(event),
    };
    let s = &state.bindings;
    let available = state.available_targets();
    let options: Vec<_> = applications.iter().filter_map(|application| {
        let target = SkillBindingTarget::application(application.id);
        available.contains(&target).then(|| (target, application.name.clone()))
    }).collect();
    let selected_names: Vec<_> = options.iter()
        .filter(|(target, _)| s.targets.contains(target))
        .map(|(_, name)| name.clone())
        .collect();
    let label = match selected_names.as_slice() {
        [] if s.busy => "安装到 · 正在读取".to_owned(),
        [] if state.awaiting_target_snapshot() => "安装到 · 待读取".to_owned(),
        [] => "安装到 · 主副本".to_owned(),
        names if names.len() == options.len() => format!("安装到 · 全部（{}）", names.len()),
        [name] => format!("安装到 · {name}"),
        [first, second] => format!("安装到 · {first}、{second}"),
        [name, others @ ..] => format!("安装到 · {name} +{}", others.len()),
    };
    let picker = || {
        let choose = |label, all, enabled: bool| {
            button(text(label).size(12))
                .padding([5, 7])
                .style(rounded_action_btn_style)
                .on_press_maybe((!s.busy && enabled).then(|| Message {
                    scope: scope.clone(),
                    catalog: state.catalog,
                    action: Action::SelectAllTargets(all),
                }))
        };
        let mut panel = column![
            row![
                text("在这些应用中启用技能").size(13).width(Length::Fill),
                choose("全选", true, selected_names.len() < options.len()),
                choose(
                    "仅主副本",
                    false,
                    !selected_names.is_empty() || state.awaiting_target_snapshot()
                ),
            ]
            .spacing(5)
            .align_y(iced::Alignment::Center)
        ]
        .spacing(8)
        .padding(6);
        let mut choices = column![].spacing(3);
        for pair in options.chunks(2) {
            let mut line = row![].spacing(8);
            for (target, name) in pair {
                let selected = s.targets.contains(target);
                let target = target.clone();
                line = line.push(
                    row![
                        icon(target.app_id, 16.0),
                        checkbox(selected)
                            .label(name.clone())
                            .text_size(13)
                            .size(15)
                            .style(style::checkbox)
                            .on_toggle_maybe((!s.busy).then_some(move |enabled| {
                                send(bindings::Event::Target(target.clone(), enabled))
                            })),
                    ]
                    .spacing(6)
                    .align_y(iced::Alignment::Center)
                    .width(Length::Fill)
                    .clip(true),
                );
            }
            if pair.len() == 1 {
                line = line.push(Space::new().width(Length::Fill));
            }
            choices = choices.push(line);
        }
        if options.is_empty() {
            panel = panel.push(
                text(if s.busy { "正在读取应用…" } else { "没有可选应用" })
                    .size(12)
                    .style(settings_muted_text_style),
            );
        } else {
            panel = panel.push(scrollable(choices).height(Length::Shrink).direction(
                scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(4).scroller_width(4),
                ),
            ));

        }
        if let Some(error) = &s.error {
            panel = panel.push(settings_error_banner(error.clone())).push(
                button(text("重试").size(13))
                    .padding([6, 10])
                    .style(rounded_action_btn_style)
                    .on_press(send(bindings::Event::Refresh)),
            );
        }
        panel
    };
    let bindings = s.selected.as_ref().map(|id| {
        let skill_name = state
            .workspace
            .inventory
            .as_ref()
            .and_then(|inventory| inventory.skills.iter().find(|skill| &skill.id == id))
            .map_or(id.as_str(), |skill| skill.name.as_str());
        let mut panel = column![
            settings_modal_header(
                text(format!("{skill_name} · 应用绑定")).size(15),
                settings_close_button(send(bindings::Event::Close))
            ),
            picker(),
        ]
        .spacing(6);
        let mut controls = row![].spacing(8);
        for (label, event) in [
            ("启用所选", bindings::Event::Bind(true)),
            ("停用所选", bindings::Event::Bind(false)),
            ("重新同步", bindings::Event::Resync),
        ] {
            controls = controls.push(
                button(text(label).size(13))
                    .padding([6, 10])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!s.busy && !s.targets.is_empty()).then(|| send(event))),
            );
        }
        panel = panel.push(controls);
        for result in &s.results {
            if let Some((_, name)) = options.iter().find(|(target, _)| target == &result.target) {
                panel = panel.push(text(format!("{name}：{:?}", result.status)).size(12));
            }
        }
        container(panel).padding(10).width(Length::Fill).style(settings_section_card_style).into()
    });
    workspace::view(state, label, picker().into(), bindings, &options, icon)
}
