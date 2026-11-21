//! Source metadata is display-only until the current local grant is verified.
use vw_api_external_session::*;
use super::{Event as ParentEvent, Scope, State, message};
use desktop_ui_controls::system_settings_common::*;
use super::Event as Message;
use crate::{Connection, Notice, error as notify_error, success as notify_success};
use iced::{Element, Length, Task, widget::{button, column, container, row, scrollable, text}};
use iced::widget::scrollable::{Direction, Scrollbar};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use vw_api_external_session::SessionMeta;

const SOURCE_PATHS: &[(&str, &[&str])] = &[
    ("Claude Code", &["~/.claude/projects"]),
    ("Codex", &["~/.codex/sessions", "~/.codex/archived_sessions"]),
    ("OpenCode", &["~/.local/share/opencode"]),
    ("Hermes", &["~/.hermes/state.db", "~/.hermes/sessions"]),
    ("Gemini", &["~/.gemini/tmp/<project>/chats"]),
    ("Pi", &["~/.pi/agent/sessions"]),
    ("OMP", &["~/.omp/agent/sessions"]),
    ("Grok Build", &["~/.grok/sessions", "~/.grok/archived_sessions"]),
    ("OpenClaw", &["~/.openclaw/agents/<agent>/sessions"]),
    ("MiniMax Code", &["~/.minimax"]),
    ("Kimi", &["~/.kimi-code/sessions"]),
    ("DSH", &["~/.dsh/sessions/<project>/<session>/session.v*.jsonl[.zstd]"]),
    ("Antigravity", &["~/.gemini/antigravity-cli/brain/<conversation>/.system_generated/logs"]),
];

#[derive(Clone, Default)]
pub(super) struct Sources {
    instance: Arc<()>,
    copied: HashSet<&'static str>,
    revealable: HashMap<&'static str, String>,
}

#[cfg(target_os = "macos")]
fn revealable_sources() -> HashMap<&'static str, String> {
    let Some(home) = std::env::home_dir() else {
        return HashMap::new();
    };
    SOURCE_PATHS.iter()
        .flat_map(|(_, paths)| paths.iter().copied())
        .filter_map(|label| {
            if label.contains('<') || label.contains('*') {
                return None;
            }
            let path = home.join(label.strip_prefix("~/")?);
            let path = local_path(path.to_str()?)?;
            std::path::Path::new(&path).try_exists().ok()?.then_some((label, path))
        })
        .collect()
}

fn msg(event: Event) -> Message { message(ParentEvent::Path(event)) }

fn location(session: &SessionMeta, kind: Location) -> Option<&str> {
    match kind {
        Location::Source => session.source_path.as_deref(),
        Location::Project => session.project_dir.as_deref(),
        Location::Identity => Some(session.identity.session_id.0.as_str()),
    }.filter(|path| !path.trim().is_empty())
}

/// Do not turn database locators, remote file URLs or relative paths into local paths.
fn local_path(value: &str) -> Option<String> {
    let path = if value.starts_with("file:") {
        let url = url::Url::parse(value).ok()?;
        if url.host_str().is_some_and(|host| host != "localhost")
            || url.query().is_some() || url.fragment().is_some() { return None; }
        url.to_file_path().ok()?.to_str()?.to_owned()
    } else { value.to_owned() };
    if !std::path::Path::new(&path).is_absolute() || path.starts_with("//")
        || path.starts_with("\\\\") || path.chars().any(char::is_control) { return None; }
    Some(path)
}

pub(super) fn update(state: &mut super::State, connection: &Connection, notice: &mut Option<Notice>, event: Event, verified_local: bool) -> Task<Message> {
    let client = connection.client.clone();
    let context = connection.context.clone();
    let project = connection.project.clone();

    if !state.open || connection.catalog_context.as_ref() != Some(&context)
        || state.host != connection.host { return Task::none(); }
    let scope = Scope { context, host: state.host.clone(), project,
        catalog: connection.generation, generation: state.generation, append: false };
    let (identity, generation, kind, reveal) = match event {
        Event::Sources(false) => {
            state.sources = None;
            return Task::none();
        }
        Event::Sources(true) => {
            let sources = Sources::default();
            #[cfg(target_os = "macos")]
            let instance = sources.instance.clone();
            state.sources = Some(sources);
            #[cfg(target_os = "macos")]
            if verified_local && client.endpoint().is_loopback() {
                return Task::perform(async move {
                    let paths = tokio::task::spawn_blocking(revealable_sources).await.unwrap_or_default();
                    msg(Event::RevealableSources(instance, paths))
                }, |message| message);
            }
            return Task::none();
        }
        Event::RevealableSources(instance, paths) => {
            if let Some(sources) = &mut state.sources
                && Arc::ptr_eq(&sources.instance, &instance)
            {
                sources.revealable = paths;
            }
            return Task::none();
        }
        Event::CopySource(path) => {
            let Some(sources) = &mut state.sources else { return Task::none(); };
            if !sources.copied.insert(path) {
                return Task::none();
            }
            let instance = sources.instance.clone();
            return iced::clipboard::write(path.to_owned()).chain(Task::perform(async move {
                crate::diagnostics::sleep::sleep(Duration::from_secs(2)).await;
                msg(Event::ResetSourceCopy(instance, path))
            }, |message| message));
        }
        Event::ResetSourceCopy(instance, path) => {
            if let Some(sources) = &mut state.sources
                && Arc::ptr_eq(&sources.instance, &instance)
            {
                sources.copied.remove(path);
            }
            return Task::none();
        }
        Event::RevealSource(label) => {
            if !cfg!(target_os = "macos") || !verified_local {
                return notify_error(notice, "此路径无法在 Finder 中显示，可以复制路径");
            }
            let Some(path) = state.sources.as_ref()
                .and_then(|sources| sources.revealable.get(label)).cloned()
            else { return Task::none(); };
            return reveal_path(client.endpoint().clone(), path, scope);
        }
        Event::Finished(other, result) => {
            if other.context != scope.context || other.host != scope.host || other.project != scope.project
                || other.catalog != scope.catalog || other.generation != scope.generation { return Task::none(); }
            return match result {
                Ok(()) => Task::none(),
                Err(_) => notify_error(notice, "无法在 Finder 中显示：请确认本机位置仍然存在且可访问"),
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
        return Task::batch([iced::clipboard::write(value), notify_success(notice, "已复制会话信息")]);
    }
    // A loopback gateway can forward to remote hosts. Also verify the private local grant.
    if !cfg!(target_os = "macos")
        || !verified_local
    {
        return notify_error(notice, "此路径无法在 Finder 中显示，可以复制路径");
    }
    let Some(path) = local_path(&value) else {
        return notify_error(notice, "此来源位置不是可在 Finder 中显示的本机绝对路径");
    };
    reveal_path(client.endpoint().clone(), path, scope)
}

fn reveal_path(endpoint: gateway_endpoint::GatewayEndpoint, path: String, scope: Scope) -> Task<Message> {
    Task::perform(async move {
        let result = desktop_config::config_file_manager::reveal_gateway_path_locally(&endpoint, &path).await;
        msg(Event::Finished(scope, result))
    }, |message| message)
}

pub(super) fn controls<'a>(state: &State, session: &'a SessionMeta, loopback: bool) -> Element<'a, Message> {
    let local = cfg!(target_os = "macos") && session.identity.target_host == local_app_auth::HOST_ID
        && loopback;
    let source = format!("来源应用：{:?} · 实例：{}",
        session.identity.provider, session.identity.instance_id);
    let mut content = column![text(source).size(12).style(settings_muted_text_style)].spacing(4);
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
        content = content.push(text("此路径可以复制。")
            .size(12).style(settings_muted_text_style));
    }
    content.into()
}

pub(super) fn source_button() -> Element<'static, Message> {
    button(text("会话记录在哪里")).padding([8, 12]).style(rounded_action_btn_style)
        .on_press(msg(Event::Sources(true))).into()
}

pub(super) fn view<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(sources) = &state.sources else { return base; };
    let close = msg(Event::Sources(false));
    let mut locations = column![].spacing(10).padding([0, 7]);
    for (index, &(app, paths)) in SOURCE_PATHS.iter().enumerate() {
        if index > 0 {
            locations = locations.push(settings_divider());
        }
        let mut group = column![row![
            desktop_ui_controls::icons::acp_agent_icon(app, 20.0),
            text(app).size(14),
        ].spacing(8).align_y(iced::Alignment::Center)].spacing(6);
        for &path in paths {
            let mut line = row![
                text(path)
                    .size(12)
                    .width(Length::Fill)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                    .style(settings_muted_text_style),
            ].spacing(8);
            if sources.revealable.contains_key(path) {
                line = line.push(button(text("在 Finder 中显示"))
                    .padding([4, 8])
                    .style(rounded_action_btn_style)
                    .on_press(msg(Event::RevealSource(path))));
            }
            let copied = sources.copied.contains(path);
            let copy = button(text(if copied { "已复制" } else { "复制路径" }))
                .padding([4, 8])
                .style(rounded_action_btn_style)
                .on_press_maybe((!copied).then(|| msg(Event::CopySource(path))));
            group = group.push(line.push(copy).align_y(iced::Alignment::Center));
        }
        locations = locations.push(group);
    }
    let header = settings_modal_header(
        text("会话记录在哪里").size(18),
        settings_close_button(close.clone()),
    );
    let card = settings_modal_card(column![
        header,
        text("以下是默认位置。修改过数据目录时，以会话详情中的来源为准。Claude Desktop 使用 Claude Code 的记录。")
            .style(settings_muted_text_style),
        scrollable(container(locations).width(Length::Fill)).height(Length::Fixed(320.0))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
    ].spacing(12)).width(Length::Fill).max_width(600);
    settings_modal_overlay_with_escape_and_backdrop_dismiss(base, close, container(card).max_width(600))
}

#[derive(Clone, Copy, Debug)]
pub enum Location { Source, Project, Identity }

#[derive(Clone, Debug)]
pub enum Event {
    Sources(bool),
    RevealableSources(Arc<()>, HashMap<&'static str, String>),
    CopySource(&'static str),
    ResetSourceCopy(Arc<()>, &'static str),
    RevealSource(&'static str),
    Copy(ExternalSessionIdentity, u64, Location),
    Reveal(ExternalSessionIdentity, u64, Location),
    Finished(Scope, Result<(), String>),
}
