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
        text(if session.capabilities.resume { "在终端中恢复" }
            else { "来源应用未提供此会话的原生恢复能力" }).size(12).style(settings_muted_text_style),
    ].spacing(4).into()
}

pub(super) fn reader_control<'a>(session: &'a SessionMeta, state: &super::State) -> Element<'a, Message> {
    let terminal = match state.resume.preferred.unwrap_or(SessionTerminal::Terminal) {
        SessionTerminal::Terminal if session.identity.target_host == local_app_auth::HOST_ID
            && cfg!(target_os = "macos") => "Terminal.app",
        SessionTerminal::Terminal => "终端",
        SessionTerminal::Iterm2 => "iTerm2",
        SessionTerminal::Ghostty => "Ghostty",
        SessionTerminal::Kitty => "Kitty",
        SessionTerminal::Wezterm => "WezTerm",
        SessionTerminal::Kaku => "Kaku",
        SessionTerminal::Otty => "Otty",
        SessionTerminal::Alacritty => "Alacritty",
        SessionTerminal::Warp => "Warp",
        SessionTerminal::Custom => "自定义终端",
    };
    let enabled = !state.busy && !state.deletion.busy() && !state.resume.busy && session.capabilities.resume;
    let identity = session.identity.clone();
    use super::list::Action;
    let menu = settings_select([Action::CopyResume, Action::Terminal], None::<Action>, move |action| {
        match action {
            Action::CopyResume => msg(Event::CopyFor(identity.clone())),
            _ => msg(Event::Preferences),
        }
    }).menu_icon(desktop_assets::assets::Icon::ChevronDown, 180.0)
        .width(Length::Fixed(30.0)).disabled(!enabled);
    row![
        button(text(format!("在 {terminal} 中恢复")).size(13)).padding([7, 11]).style(primary_action_btn_style)
            .on_press_maybe(enabled.then(|| msg(Event::Open(session.identity.clone())))),
        menu,
    ].spacing(3).align_y(iced::Alignment::Center).into()
}

/// Compact actions remain separate from the body read target.
pub fn quick_controls(session: &SessionMeta, enabled: bool) -> Element<'_, Message> {
    use iced::widget::{tooltip, tooltip::Position};
    row![
        tooltip(button(text("↗")).padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((enabled && session.capabilities.resume).then(|| msg(Event::Open(session.identity.clone())))),
            text("在终端中恢复"), Position::Top),
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
                        notify_success(notice, "已复制恢复命令，请在会话来源环境中运行")])
                }
                Err(error) => notify_error(notice, super::request_error("恢复命令获取", &error)),
                Ok(_) => notify_error(notice, "无法复制恢复命令"),
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
                Ok(response) if response.completed => notify_success(notice, "恢复进程已完成"),
                Err(error) => notify_error(notice, super::request_error("会话恢复", &error)),
                Ok(_) => notify_error(notice, "恢复未完成，请检查终端后重试"),
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
        (SessionTerminal::Warp, "Warp"), (SessionTerminal::Custom, "自定义终端"),
    ] {
        let selected = state.resume.terminal == Some(terminal);
        choices = choices.push(button(text(if selected { format!("● {label}") } else { label.into() }))
            .padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe((!state.resume.busy).then(|| msg(Event::Terminal(terminal)))));
    }
    let card = settings_modal_card(column![
        row![text(if state.resume.preferences { "首选终端" } else { "恢复外部会话" }).size(18), settings_close_button(close.clone())].spacing(12),
        text("首选终端仅在当前工作区有效；更换授权作用域后重置。").size(12).style(settings_muted_text_style),
        text("支持已安装的 macOS 终端。自定义终端须由管理员预先配置。").style(settings_muted_text_style),
        choices.wrap(),
        button(text("复制恢复命令")).padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe((!state.resume.busy && !state.resume.preferences).then(|| msg(Event::Copy))),
        text("恢复命令适用于会话来源环境。依赖账号凭据的会话无法复制恢复命令。").style(settings_muted_text_style),
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
