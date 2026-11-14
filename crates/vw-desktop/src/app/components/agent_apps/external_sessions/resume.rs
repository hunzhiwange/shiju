//! Terminal choices are sent to the host; desktop never constructs native commands.
use super::{message, Scope};
use crate::app::{App, Message, components::system_settings_common::*, state::AgentAppsContext};
use iced::{Element, Length, Task, widget::{button, column, container, row, text}};
use vw_gateway_client::{AgentAppsReadError, vw_api_types::external_session::*};

#[derive(Clone, Default)]
pub(super) struct State {
    identity: Option<ExternalSessionIdentity>,
    terminal: Option<SessionTerminal>,
    pub(super) busy: bool,
    generation: u64,
}
impl State {
    pub(super) fn reset(&mut self) {
        *self = Self { generation: self.generation.wrapping_add(1), ..Self::default() };
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Open(ExternalSessionIdentity),
    Close,
    Terminal(SessionTerminal),
    Launch,
    Copy,
    Command(Scope, ExternalSessionIdentity, u64, Result<SessionResumeCommandResponse, AgentAppsReadError>),
    Finished(Scope, ExternalSessionIdentity, u64, Result<SessionResumeResponse, AgentAppsReadError>),
}
fn msg(event: Event) -> Message { message(super::Event::Resume(event)) }

pub(super) fn control(session: &SessionMeta, enabled: bool) -> Element<'_, Message> {
    column![button(text("恢复会话")).padding([8, 12]).style(rounded_action_btn_style)
        .on_press_maybe((enabled && session.capabilities.resume)
            .then(|| msg(Event::Open(session.identity.clone())))),
        text(if session.capabilities.resume { "在授权主机的终端中恢复" }
            else { "来源应用未提供此会话的原生恢复能力" }).size(12).style(settings_muted_text_style),
    ].spacing(4).into()
}

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let Some(project) = app.project_id.clone() else { return Task::none(); };
    let state = &mut app.apps_catalog.external_sessions;
    if !state.open || app.apps_catalog.context.as_ref() != Some(&context)
        || state.host != app.apps_catalog.host || app.apps_catalog.token.is_empty() { return Task::none(); }
    let scope = Scope { context, host: state.host.clone(), project,
        catalog: app.apps_catalog.generation, generation: state.generation, append: false };
    match event {
        Event::Close => {
            state.resume.identity = None;
            state.resume.busy = false;
            state.resume.generation = state.resume.generation.wrapping_add(1);
        }
        Event::Command(other, identity, generation, result) => {
            if other.context != scope.context || other.host != scope.host || other.project != scope.project
                || other.catalog != scope.catalog || other.generation != scope.generation
                || state.resume.identity.as_ref() != Some(&identity)
                || state.resume.generation != generation || !state.resume.busy
                || !state.page.as_ref().is_some_and(|page| page.sessions.iter()
                    .any(|session| session.identity == identity && session.capabilities.resume)) { return Task::none(); }
            state.resume.busy = false;
            return match result {
                Ok(response) if !response.command.is_empty()
                    && !response.command.contains(&app.apps_catalog.token) => {
                    Task::batch([iced::clipboard::write(response.command),
                        app.show_success_toast("已复制受控命令，请仅在目标授权主机终端运行")])
                }
                _ => app.show_error_toast("命令不可复制：请检查主机授权；依赖账号凭据的恢复不支持复制"),
            };
        }
        Event::Finished(other, identity, generation, result) => {
            if other.context != scope.context || other.host != scope.host || other.project != scope.project
                || other.catalog != scope.catalog || other.generation != scope.generation
                || state.resume.identity.as_ref() != Some(&identity)
                || state.resume.generation != generation || !state.resume.busy
                || !state.page.as_ref().is_some_and(|page| page.sessions.iter()
                    .any(|session| session.identity == identity && session.capabilities.resume)) { return Task::none(); }
            state.resume.busy = false;
            return match result {
                Ok(response) if response.completed => app.show_success_toast("主机恢复进程已完成"),
                _ => app.show_error_toast("恢复未完成，请检查授权主机和终端后重试；可尝试复制受控命令"),
            };
        }
        _ if state.resume.busy || state.deletion.busy() || state.busy => return Task::none(),
        Event::Open(identity) => {
            if identity.target_host != scope.host || identity.scope.project_id().as_ref() != scope.project.as_str()
                || !state.page.as_ref().is_some_and(|page| page.sessions.iter()
                    .any(|session| session.identity == identity && session.capabilities.resume)) { return Task::none(); }
            state.resume.generation = state.resume.generation.wrapping_add(1);
            state.resume.identity = Some(identity);
            state.resume.terminal = Some(SessionTerminal::Terminal);
        }
        Event::Terminal(terminal) => {
            if state.resume.identity.is_some() { state.resume.terminal = Some(terminal); }
        }
        Event::Launch | Event::Copy => {
            let copy = matches!(event, Event::Copy);
            let (Some(identity), Some(terminal)) = (state.resume.identity.clone(), state.resume.terminal) else { return Task::none(); };
            if identity.target_host != scope.host || identity.scope.project_id().as_ref() != scope.project.as_str()
                || !state.page.as_ref().is_some_and(|page| page.sessions.iter()
                    .any(|session| session.identity == identity && session.capabilities.resume)) { return Task::none(); }
            state.resume.busy = true;
            state.resume.generation = state.resume.generation.wrapping_add(1);
            let generation = state.resume.generation;
            let token = app.apps_catalog.token.clone();
            return Task::perform(async move {
                let request = SessionResumeRequest { identity: identity.clone(), terminal };
                if copy {
                    let result = client.external_session_resume_command(&request, &token).await;
                    return msg(Event::Command(scope, identity, generation, result));
                }
                let result = client.external_session_resume(&request, &token).await;
                msg(Event::Finished(scope, identity, generation, result))
            }, |message| message);
        }
    }
    Task::none()
}

pub(super) fn view<'a>(state: &'a super::State, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(identity) = &state.resume.identity else { return base; };
    let close = msg(Event::Close);
    let mut choices = row![].spacing(8).wrap();
    for (terminal, label) in [
        (SessionTerminal::Terminal, "Terminal"), (SessionTerminal::Iterm2, "iTerm2"),
        (SessionTerminal::Ghostty, "Ghostty"), (SessionTerminal::Kitty, "Kitty"),
        (SessionTerminal::Wezterm, "WezTerm"), (SessionTerminal::Kaku, "Kaku"),
        (SessionTerminal::Otty, "Otty"), (SessionTerminal::Alacritty, "Alacritty"),
        (SessionTerminal::Warp, "Warp"), (SessionTerminal::Custom, "主机自定义终端"),
    ] {
        let selected = state.resume.terminal == Some(terminal);
        choices = choices.push(button(text(if selected { format!("● {label}") } else { label.into() }))
            .padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe((!state.resume.busy).then(|| msg(Event::Terminal(terminal)))));
    }
    let card = settings_modal_card(column![
        row![text("恢复外部会话").size(18), settings_close_button(close.clone())].spacing(12),
        text(format!("目标主机：{}", identity.target_host)),
        text("选择目标主机终端；自定义终端须由主机管理员预先授权。").style(settings_muted_text_style),
        choices,
        button(text("复制恢复命令")).padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe((!state.resume.busy).then(|| msg(Event::Copy))),
        text("命令仅适用于目标主机；依赖账号凭据的恢复不可复制。").style(settings_muted_text_style),
        row![button(text("关闭")).padding([8, 12]).style(rounded_action_btn_style).on_press(close.clone()),
            button(text(if state.resume.busy { "正在恢复…" } else { "在所选终端恢复" }))
                .padding([8, 12]).style(primary_action_btn_style)
                .on_press_maybe((!state.resume.busy).then(|| msg(Event::Launch)))].spacing(8),
    ].spacing(16)).width(Length::Fixed(560.0));
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
