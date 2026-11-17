//! Third-party skill entry. The host routes scoped directory actions to existing
//! modals and executes binding effects with the same scope attached to replies.
use desktop_provider_apps::Scope;
use desktop_ui_controls::system_settings_common::*;
use iced::{Element, Length, widget::{button, checkbox, column, row, text}};
use crate::update::{Input, bindings_view as bindings};

#[derive(Clone, Copy, Debug)]
pub enum DirectoryAction {
    Repositories,
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
}

pub enum Effect {
    None,
    Directory { scope: Scope, catalog: u64, action: DirectoryAction },
    Binding { scope: Scope, catalog: u64, effect: bindings::Effect },
}

impl State {
    /// Synchronize before rendering and dispatch. Native skill state is never
    /// borrowed or mutated by this entry.
    pub fn sync(&mut self, scope: Option<Scope>, catalog: u64) {
        if self.scope != scope || self.catalog != catalog {
            self.bindings = bindings::State::default();
            self.scope = scope;
            self.catalog = catalog;
        }
    }

    pub fn scope(&self) -> Option<&Scope> { self.scope.as_ref() }
}

pub fn update(state: &mut State, input: &Input, message: Message) -> Effect {
    if state.scope.as_ref() != Some(&message.scope)
        || input.context.as_ref() != Some(&message.scope.context)
        || input.host != message.scope.host
        || state.catalog != message.catalog
        || state.catalog != input.catalog
        || input.directory != message.scope.context.project
    {
        return Effect::None;
    }
    let scope = message.scope;
    let mut event = match message.action {
        Action::Binding(event) => event,
        Action::Directory(action) => return Effect::Directory { scope, catalog: state.catalog, action },
    };
    // Global preferences contain other applications. This page deliberately
    // changes bindings only, leaving shared preference editing to its owner.
    if matches!(event, bindings::Event::Save) {
        return Effect::Binding { scope, catalog: state.catalog, effect: bindings::Effect::Error(
            "请在全局应用绑定中保存目标偏好".into(),
        ) };
    }
    if let bindings::Event::Target(target, _) = &event {
        if target.app_id != scope.app { return Effect::None; }
    }
    if let bindings::Event::Completed(_, Ok(data)) = &mut event {
        if let Some(snapshot) = &mut data.snapshot {
            snapshot.targets.retain(|target| target.app_id == scope.app);
            snapshot.capabilities.retain(|capability| capability.app_id == scope.app);
        }
        data.results.retain(|item| item.target.app_id == scope.app);
    }
    state.bindings.targets.retain(|target| target.app_id == scope.app);
    if matches!(event, bindings::Event::Bind(_) | bindings::Event::Resync) {
        let supported = state.bindings.snapshot.as_ref().is_some_and(|snapshot|
            snapshot.capabilities.iter().any(|capability|
                capability.app_id == scope.app && capability.supported));
        if !supported || state.bindings.targets.iter().any(|target| !input.targets.contains(target)) {
            return Effect::Binding { scope, catalog: state.catalog, effect: bindings::Effect::Error(
                "当前应用或安装实例不支持技能绑定，请刷新应用目录".into(),
            ) };
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
        return settings_error_banner("请先选择已连接主机上的应用".to_owned()).into();
    };
    let message = |action| Message { scope: scope.clone(), catalog: state.catalog, action };
    let mut body = column![
        text("第三方 Skills").size(28),
        text(format!("应用：{:?} · 主机：{}", scope.app, scope.host)),
        text("技能目录由主机共享；绑定操作仅作用于当前应用，不改变原生技能配置。"),
    ].spacing(16).width(Length::Fill);
    let mut actions = row![].spacing(8);
    for (label, action) in [
        ("仓库管理", DirectoryAction::Repositories), ("导入技能", DirectoryAction::Import),
        ("详情与安装", DirectoryAction::Install), ("技能市场", DirectoryAction::Market),
        ("更新与卸载", DirectoryAction::Updates), ("任务进度", DirectoryAction::Jobs),
        ("存储与同步", DirectoryAction::Storage), ("备份与恢复", DirectoryAction::Backup),
    ] {
        actions = actions.push(button(label).padding([8, 10]).style(rounded_action_btn_style)
            .on_press(message(Action::Directory(action))));
    }
    body = body.push(actions.wrap());
    let s = &state.bindings;
    body = body.push(button("刷新应用绑定").padding([8, 10]).style(rounded_action_btn_style)
        .on_press_maybe((!s.busy).then(|| message(Action::Binding(
            if s.open { bindings::Event::Refresh } else { bindings::Event::Open }
        )))));
    if let Some(inventory) = &s.inventory {
        for skill in inventory.skills.iter().filter(|skill| skill.present) {
            body = body.push(button(text(format!("{}{}",
                if s.selected.as_ref() == Some(&skill.id) { "✓ " } else { "" }, skill.name)))
                .padding([8, 10]).style(rounded_action_btn_style)
                .on_press_maybe((!s.busy).then(|| message(Action::Binding(bindings::Event::Select(skill.id.clone()))))));
        }
    }
    let capability = s.snapshot.as_ref().and_then(|snapshot|
        snapshot.capabilities.iter().find(|item| item.app_id == scope.app));
    if let Some(capability) = capability {
        body = body.push(text(capability.reason.clone()));
        if capability.supported {
            if let Some(observation) = applications.iter().find(|item| item.entry.id == scope.app)
                .and_then(|item| item.observation)
            {
                for installation in observation.installations.iter().filter(|item| item.target_host_id == scope.host) {
                    let target = vw_api_app_skills::bindings_api::SkillBindingTarget {
                        app_id: scope.app, installation_id: installation.id.0.clone(),
                    };
                    let selected = s.targets.contains(&target);
                    let scope = scope.clone();
                    body = body.push(checkbox(selected).label(installation.id.0.clone())
                        .text_size(SETTINGS_CONTROL_TEXT_SIZE).style(settings_checkbox_style)
                        .on_toggle_maybe((!s.busy).then_some(move |enabled| Message {
                            scope: scope.clone(), catalog: state.catalog, action: Action::Binding(bindings::Event::Target(target.clone(), enabled)),
                        })));
                }
            } else { body = body.push(text("应用实例尚未读取，请在应用设置中刷新")); }
        }
    }
    let supported = capability.is_some_and(|item| item.supported);
    let can_bind = supported && !s.busy && !s.targets.is_empty();
    let mut controls = row![].spacing(8);
    for (label, event, enabled) in [
        ("启用所选", bindings::Event::Bind(true), can_bind && s.selected.is_some()),
        ("停用所选", bindings::Event::Bind(false), can_bind && s.selected.is_some()),
        ("重新同步", bindings::Event::Resync, can_bind),
        ("取消选择", bindings::Event::Close, !s.busy),
    ] {
        controls = controls.push(button(label).padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe(enabled.then(|| message(Action::Binding(event)))));
    }
    body = body.push(controls);
    if s.busy { body = body.push(text("正在读取或写入，请稍候…")); }
    for result in &s.results {
        body = body.push(text(format!("{} / {}：{:?}", result.target.installation_id, result.directory, result.status)));
    }
    if let Some(error) = &s.error { body = body.push(settings_error_banner(error.clone())); }
    body.into()
}
