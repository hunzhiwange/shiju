//! Source metadata is display-only until the current local grant is verified.
use super::{Event as ParentEvent, Scope, State, message};
use crate::app::{App, Message, components::system_settings_common::*, state::AgentAppsContext};
use iced::{Element, Length, Task, widget::{button, column, container, row, scrollable, text}};
use iced::widget::scrollable::{Direction, Scrollbar};
use vw_gateway_client::vw_api_types::external_session::{ExternalSessionIdentity, SessionMeta};

#[derive(Clone, Copy, Debug)]
pub enum Location { Source, Project }

#[derive(Clone, Debug)]
pub enum Event {
    Sources(bool),
    Copy(ExternalSessionIdentity, u64, Location),
    Reveal(ExternalSessionIdentity, u64, Location),
    Finished(Scope, Result<(), String>),
}
fn msg(event: Event) -> Message { message(ParentEvent::Path(event)) }

fn location(session: &SessionMeta, kind: Location) -> Option<&str> {
    match kind {
        Location::Source => session.source_path.as_deref(),
        Location::Project => session.project_dir.as_deref(),
    }.filter(|path| !path.trim().is_empty())
}

/// Do not turn database locators, remote file URLs or relative paths into local paths.
fn local_path(value: &str) -> Option<String> {
    let path = if value.starts_with("file:") {
        let url = reqwest::Url::parse(value).ok()?;
        if url.host_str().is_some_and(|host| host != "localhost")
            || url.query().is_some() || url.fragment().is_some() { return None; }
        url.to_file_path().ok()?.to_str()?.to_owned()
    } else { value.to_owned() };
    if !std::path::Path::new(&path).is_absolute() || path.starts_with("//")
        || path.starts_with("\\\\") || path.chars().any(char::is_control) { return None; }
    Some(path)
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
    let (identity, generation, kind, reveal) = match event {
        Event::Sources(open) => { state.sources_open = open; return Task::none(); }
        Event::Finished(other, result) => {
            if other.context != scope.context || other.host != scope.host || other.project != scope.project
                || other.catalog != scope.catalog || other.generation != scope.generation { return Task::none(); }
            return match result {
                Ok(()) => Task::none(),
                Err(_) => app.show_error_toast("无法在 Finder 中显示：请确认本机位置仍然存在且可访问"),
            };
        }
        Event::Copy(identity, generation, kind) => (identity, generation, kind, false),
        Event::Reveal(identity, generation, kind) => (identity, generation, kind, true),
    };
    if generation != scope.generation || identity.target_host != scope.host
        || identity.scope.tenant_scope() != client.endpoint().tenant_scope()
        || identity.scope.project_id().as_ref() != scope.project.as_str() { return Task::none(); }
    let Some(value) = state.page.as_ref().and_then(|page| page.sessions.iter()
        .find(|session| session.identity == identity)).and_then(|session| location(session, kind))
        .map(str::to_owned) else { return Task::none(); };
    if !reveal {
        return Task::batch([iced::clipboard::write(value), app.show_success_toast("已复制来源主机路径")]);
    }
    // A loopback gateway can forward to remote hosts. Also verify the private local grant.
    if !cfg!(target_os = "macos")
        || crate::app::components::system_settings_apps::local::launch_directory(
            &client, &identity.target_host, &app.apps_catalog.token, None).is_none()
    {
        return app.show_error_toast("只有已确认属于当前桌面的路径才能在 Finder 中显示；请复制到来源主机使用");
    }
    let Some(path) = local_path(&value) else {
        return app.show_error_toast("此来源位置不是可在 Finder 中显示的本机绝对路径");
    };
    Task::perform(async move {
        let result = crate::app::config::reveal_gateway_path_locally(client.endpoint(), &path).await;
        msg(Event::Finished(scope, result))
    }, |message| message)
}

pub(super) fn controls<'a>(state: &State, session: &'a SessionMeta) -> Element<'a, Message> {
    let local = cfg!(target_os = "macos") && session.identity.target_host == vw_shared::local_apps::HOST_ID
        && crate::app::config::gateway_client().is_ok_and(|client| client.endpoint().is_loopback());
    let mut content = column![text(format!("来源应用：{:?} · 实例：{} · 执行主机：{}",
        session.identity.provider, session.identity.instance_id, session.identity.target_host))
        .size(12).style(settings_muted_text_style)].spacing(4);
    for (kind, label) in [(Location::Project, "项目位置"), (Location::Source, "会话来源")] {
        let Some(path) = location(session, kind) else { continue; };
        let identity = session.identity.clone();
        content = content.push(text(format!("{label}：{path}")).size(12).style(settings_muted_text_style));
        content = content.push(row![
            button(text(format!("复制{label}"))).padding([8, 12]).style(rounded_action_btn_style)
                .on_press(msg(Event::Copy(identity.clone(), state.generation, kind))),
            button(text("在 Finder 中显示")).padding([8, 12]).style(rounded_action_btn_style)
                .on_press_maybe((local && local_path(path).is_some()).then(||
                    msg(Event::Reveal(identity, state.generation, kind)))),
        ].spacing(8).wrap());
    }
    if !local {
        content = content.push(text("远程或未确认同机的来源位置仅供展示和复制，请在来源主机使用。")
            .size(12).style(settings_muted_text_style));
    }
    content.into()
}

pub(super) fn source_button() -> Element<'static, Message> {
    button(text("会话记录在哪里")).padding([8, 12]).style(rounded_action_btn_style)
        .on_press(msg(Event::Sources(true))).into()
}

pub(super) fn view<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    if !state.sources_open { return base; }
    let close = msg(Event::Sources(false));
    let mut locations = column![].spacing(10).padding([0, 7]);
    for (app, paths) in [
        ("Claude Code", "~/.claude/projects"),
        ("Codex", "~/.codex/sessions；~/.codex/archived_sessions"),
        ("OpenCode", "~/.local/share/opencode"),
        ("Hermes", "~/.hermes/state.db；~/.hermes/sessions"),
        ("Gemini", "~/.gemini/tmp/<project>/chats"),
        ("Pi", "~/.pi/agent/sessions"),
        ("OMP", "~/.omp/agent/sessions"),
        ("Grok Build", "~/.grok/sessions；~/.grok/archived_sessions"),
        ("OpenClaw", "~/.openclaw/agents/<agent>/sessions"),
        ("MiniMax Code", "~/.minimax"),
        ("Kimi", "~/.kimi-code/sessions"),
        ("DSH", "~/.dsh/sessions/<project>/<session>/session.v*.jsonl[.zstd]"),
        ("Antigravity", "~/.gemini/antigravity-cli/brain/<conversation>/.system_generated/logs"),
    ] {
        locations = locations.push(column![text(app).size(14),
            text(paths).size(12).style(settings_muted_text_style)].spacing(4));
    }
    let card = settings_modal_card(column![
        row![text("会话记录在哪里").size(18), settings_close_button(close.clone())].spacing(12),
        text(format!("当前执行主机：{}", state.host)),
        text("以下是来源主机上的默认位置；修改过配置或数据目录时，以授权实例及会话实际来源为准。Claude Desktop 使用 Claude Code 的记录。")
            .style(settings_muted_text_style),
        scrollable(container(locations).width(Length::Fill)).height(Length::Fixed(320.0))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
        button(text("关闭")).padding([8, 12]).style(rounded_action_btn_style).on_press(close.clone()),
    ].spacing(12)).width(Length::Fixed(600.0));
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
