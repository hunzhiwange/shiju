//! Terminal choices are sent to the host; desktop never constructs native commands.
use gateway_external_session::AgentAppsReadError;
use super::{message, Scope};
use desktop_ui_controls::system_settings_common::*;
use super::Event as Message;
use crate::{Connection, Notice, error as notify_error, success as notify_success};
use iced::{Element, Length, Task, widget::{button, column, container, row, text}};
use vw_api_external_session::*;

fn msg(event: Event) -> Message { message(super::Event::Resume(event)) }

pub(super) fn control(session: &SessionMeta, enabled: bool) -> Element<'_, Message> {
    column![button(text("恢复会话")).padding([8, 12]).style(rounded_action_btn_style)
        .on_press_maybe((enabled && session.capabilities.resume)
            .then(|| msg(Event::Open(session.identity.clone())))),
        text(if session.capabilities.resume { "在授权主机的终端中恢复" }
            else { "来源应用未提供此会话的原生恢复能力" }).size(12).style(settings_muted_text_style),
    ].spacing(4).into()
}

/// Compact actions remain separate from the body read target.
pub fn quick_controls(session: &SessionMeta, enabled: bool) -> Element<'_, Message> {
    use iced::widget::{tooltip, tooltip::Position};
    row![
        tooltip(button(text("↗")).padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((enabled && session.capabilities.resume).then(|| msg(Event::Open(session.identity.clone())))),
            text("在授权主机终端恢复"), Position::Top),
        tooltip(button(text("⧉")).padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((enabled && session.capabilities.resume).then(|| msg(Event::CopyFor(session.identity.clone())))),
            text("复制受控恢复命令"), Position::Top),
    ].spacing(4).into()
}

pub(super) fn update(state: &mut super::State, connection: &Connection, notice: &mut Option<Notice>, event: Event) -> Task<Message> {
    let client = connection.client.clone();
    let context = connection.context.clone();
    let project = connection.project.clone();

    if !state.open || connection.catalog_context.as_ref() != Some(&context)
        || state.host != connection.host { return Task::none(); }
    let scope = Scope { context, host: state.host.clone(), project,
        catalog: connection.generation, generation: state.generation, append: false };
    match event {
        Event::Close => {
            state.resume.preferences = false;
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
                    && !response.command.contains(&connection.token) => {
                    Task::batch([iced::clipboard::write(response.command),
                        notify_success(notice, "已复制受控命令，请仅在目标授权主机终端运行")])
                }
                _ => notify_error(notice, "命令不可复制：请检查主机授权；依赖账号凭据的恢复不支持复制"),
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
                Ok(response) if response.completed => notify_success(notice, "主机恢复进程已完成"),
                _ => notify_error(notice, "恢复未完成，请检查授权主机和终端后重试；可尝试复制受控命令"),
            };
        }
        _ if state.resume.busy || state.deletion.busy() || state.busy => return Task::none(),
        Event::Preferences => {
            state.resume.preferences = true;
            state.resume.identity = None;
            state.resume.terminal = state.resume.preferred.or(Some(SessionTerminal::Terminal));
        }
        Event::CopyFor(identity) => {
            let _ = update(state, connection, notice, Event::Open(identity.clone()));
            if state.resume.identity.as_ref() != Some(&identity) { return Task::none(); }
            return update(state, connection, notice, Event::Copy);
        }
        Event::Open(identity) => {
            if identity.target_host != scope.host || identity.scope.project_id().as_ref() != scope.project.as_str()
                || !state.page.as_ref().is_some_and(|page| page.sessions.iter()
                    .any(|session| session.identity == identity && session.capabilities.resume)) { return Task::none(); }
            state.resume.generation = state.resume.generation.wrapping_add(1);
            state.resume.identity = Some(identity);
            state.resume.preferences = false;
            state.resume.terminal = state.resume.preferred.or(Some(SessionTerminal::Terminal));
        }
        Event::Terminal(terminal) => {
            if state.resume.identity.is_some() || state.resume.preferences {
                state.resume.terminal = Some(terminal);
                state.resume.preferred = Some(terminal);
            }
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
            let token = connection.token.clone();
            return Task::perform(async move {
                let request = SessionResumeRequest { identity: identity.clone(), terminal };
                if copy {
                    let result = gateway_external_session::GatewayExternalSessionClient::new(client.transport().clone()).external_session_resume_command(&request, &token).await;
                    return msg(Event::Command(scope, identity, generation, result));
                }
                let result = gateway_external_session::GatewayExternalSessionClient::new(client.transport().clone()).external_session_resume(&request, &token).await;
                msg(Event::Finished(scope, identity, generation, result))
            }, |message| message);
        }
    }
    Task::none()
}

pub(super) fn view<'a>(state: &'a super::State, base: Element<'a, Message>) -> Element<'a, Message> {
    if state.resume.identity.is_none() && !state.resume.preferences { return base; }
    let close = msg(Event::Close);
    let mut choices = row![].spacing(8);
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
        row![text(if state.resume.preferences { "首选终端" } else { "恢复外部会话" }).size(18), settings_close_button(close.clone())].spacing(12),
        text(format!("目标主机：{}", state.host)),
        text("首选终端仅在当前工作区有效；更换授权作用域后重置。").size(12).style(settings_muted_text_style),
        text("当前主机恢复服务支持 macOS 终端；安装情况由授权主机校验。自定义终端须由主机管理员预先授权。").style(settings_muted_text_style),
        choices.wrap(),
        button(text("复制恢复命令")).padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe((!state.resume.busy && !state.resume.preferences).then(|| msg(Event::Copy))),
        text("命令仅适用于目标主机；依赖账号凭据的恢复不可复制。").style(settings_muted_text_style),
        row![button(text("关闭")).padding([8, 12]).style(rounded_action_btn_style).on_press(close.clone()),
            button(text(if state.resume.busy { "正在恢复…" } else { "在所选终端恢复" }))
                .padding([8, 12]).style(primary_action_btn_style)
                .on_press_maybe((!state.resume.busy && !state.resume.preferences).then(|| msg(Event::Launch)))].spacing(8),
    ].spacing(16)).width(Length::Fill).max_width(560);
    settings_modal_overlay_with_escape_and_backdrop_dismiss(base, close, container(card).max_width(560))
}

#[derive(Clone, Default)]
pub struct State {
    pub preferences: bool,
    pub preferred: Option<SessionTerminal>,
    pub identity: Option<ExternalSessionIdentity>,
    pub terminal: Option<SessionTerminal>,
    pub busy: bool,
    pub generation: u64,
}
impl State {
    pub fn reset(&mut self) {
        *self = Self { preferred: self.preferred, generation: self.generation.wrapping_add(1), ..Self::default() };
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Preferences,
    CopyFor(ExternalSessionIdentity),
    Open(ExternalSessionIdentity),
    Close,
    Terminal(SessionTerminal),
    Launch,
    Copy,
    Command(Scope, ExternalSessionIdentity, u64, Result<SessionResumeCommandResponse, AgentAppsReadError>),
    Finished(Scope, ExternalSessionIdentity, u64, Result<SessionResumeResponse, AgentAppsReadError>),
}
